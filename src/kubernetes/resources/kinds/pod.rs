use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;

use crate::{
    app::lists::{Column, Header, NAMESPACE},
    kubernetes::resources::ResourceData,
};

/// Returns [`ResourceData`] for the `pod` kubernetes resource
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let ready = status["containerStatuses"].as_array().map(|c| get_ready(c));
    let phase = status["phase"].as_str().map(|s| s.to_owned());
    let restarts = status["containerStatuses"].as_array().map(|c| get_restarts(c));
    let is_completed = if let Some(ph) = &phase { ph == "Succeeded" } else { false };

    let ready_str;
    let is_ready;
    if let Some(ready) = ready {
        ready_str = Some(ready.0);
        is_ready = ready.1;
    } else {
        ready_str = None;
        is_ready = false;
    }

    let is_terminating = !is_ready
        && !is_completed
        && status["containerStatuses"]
            .as_array()
            .map(|c| any_terminated(c))
            .unwrap_or(false);

    let values = [
        restarts.map(|r| r.to_string()),
        ready_str,
        if is_terminating {
            Some("Terminating".to_owned())
        } else {
            phase
        },
        status["podIP"].as_str().map(|s| s.to_owned()),
    ];

    ResourceData {
        extra_values: Box::new(values),
        is_job: has_job_reference(object),
        is_completed,
        is_ready,
        is_terminating,
    }
}

/// Returns [`Header`] for the `pod` kubernetes resource
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::fixed("RESTARTS", 3, true),
            Column::fixed("READY", 7, false),
            Column::fixed("STATUS", 12, false),
            Column::bound("IP", 11, 16, false),
        ])),
    )
}

fn get_restarts(containers: &Vec<Value>) -> u16 {
    containers
        .iter()
        .map(|c| c["restartCount"].as_u64().unwrap_or(0))
        .sum::<u64>() as u16
}

fn get_ready(containers: &Vec<Value>) -> (String, bool) {
    let ready = containers
        .iter()
        .filter(|c| c["ready"].as_bool().unwrap_or_default() == true)
        .count();

    (format!("{}/{}", ready, containers.len()), ready == containers.len())
}

fn any_terminated(containers: &Vec<Value>) -> bool {
    containers.iter().any(|c| c.get("terminated").is_some())
}

fn has_job_reference(object: &DynamicObject) -> bool {
    if let Some(references) = &object.metadata.owner_references {
        for reference in references {
            if reference.kind == "Job" {
                return true;
            }
        }
    }

    false
}
