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
    let waiting = status["containerStatuses"]
        .as_array()
        .and_then(|c| get_first_waiting_reason(c));
    let restarts = status["containerStatuses"].as_array().map(|c| get_restarts(c));
    let is_completed = if let Some(ph) = &phase { ph == "Succeeded" } else { false };
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let (ready_str, is_ready) = if let Some(ready) = ready {
        (Some(ready.0), ready.1)
    } else {
        (None, false)
    };

    let values = [
        ResourceValue::numeric(restarts.map(|r| r.to_string()), 5),
        ready_str.into(),
        if is_terminating {
            "Terminating".into()
        } else if waiting.is_some() {
            waiting.into()
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
            Column::bound("STATUS", 10, 20, false),
            Column::bound("IP", 11, 16, false),
        ])),
        Rc::new([' ', 'N', 'R', 'E', 'S', 'I', 'A']),
    )
}

fn get_restarts(containers: &[Value]) -> u64 {
    containers
        .iter()
        .map(|c| c["restartCount"].as_u64().unwrap_or(0))
        .sum::<u64>()
}

fn get_ready(containers: &[Value]) -> (String, bool) {
    let ready = containers.iter().filter(|c| c["ready"].as_bool().unwrap_or_default()).count();

    (format!("{}/{}", ready, containers.len()), ready == containers.len())
}

fn get_first_waiting_reason(containers: &[Value]) -> Option<String> {
    for c in containers {
        if let Some(reason) = c
            .get("state")
            .and_then(|s| s.as_object())
            .and_then(|s| s.get("waiting"))
            .and_then(|w| w.as_object())
            .and_then(|w| w.get("reason"))
            .and_then(|r| r.as_str())
        {
            return Some(reason.to_owned());
        }
    }

    None
}

fn has_job_reference(object: &DynamicObject) -> bool {
    if let Some(references) = &object.metadata.owner_references {
        references.iter().any(|r| r.kind == "Job")
    } else {
        false
    }
}
