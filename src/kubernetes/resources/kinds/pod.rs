use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    app::lists::{Column, Header, NAMESPACE},
    kubernetes::resources::{ResourceData, ResourceValue},
};

/// Returns [`ResourceData`] for the `pod` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let ready = status["containerStatuses"].as_array().map(|c| get_ready(c));
    let phase = status["phase"].as_str().map(|s| s.to_owned());
    let restarts = status["containerStatuses"].as_array().map(|c| get_restarts(c));
    let is_completed = if let Some(ph) = &phase { ph == "Succeeded" } else { false };
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let ready_str;
    let is_ready;
    if let Some(ready) = ready {
        ready_str = Some(ready.0);
        is_ready = ready.1;
    } else {
        ready_str = None;
        is_ready = false;
    }

    let values = [
        ResourceValue::numeric(restarts.map(|r| r.to_string()), 5),
        ready_str.into(),
        if is_terminating {
            Some("Terminating".to_owned()).into()
        } else {
            phase.into()
        },
        status["podIP"].as_str().map(|s| s.to_owned()).into(),
    ];

    ResourceData {
        extra_values: Box::new(values),
        is_job: has_job_reference(object),
        is_completed,
        is_ready: if is_terminating { false } else { is_ready },
        is_terminating,
    }
}

/// Returns [`Header`] for the `pod` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::fixed("RESTARTS", 3, true),
            Column::fixed("READY", 7, false),
            Column::fixed("STATUS", 12, false),
            Column::bound("IP", 11, 16, false),
        ])),
        Rc::new([' ', 'N', 'R', 'E', 'S', 'I', 'A']),
    )
}

fn get_restarts(containers: &[Value]) -> u16 {
    containers
        .iter()
        .map(|c| c["restartCount"].as_u64().unwrap_or(0))
        .sum::<u64>() as u16
}

fn get_ready(containers: &[Value]) -> (String, bool) {
    let ready = containers.iter().filter(|c| c["ready"].as_bool().unwrap_or_default()).count();

    (format!("{}/{}", ready, containers.len()), ready == containers.len())
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
