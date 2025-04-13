use k8s_openapi::serde_json::Value;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the pod's `container`.
pub fn data(container: &Value, status: Option<&Value>, is_init_container: bool, is_terminating: bool) -> ResourceData {
    let image = container["image"].as_str().map(|s| s.to_owned());
    let restarts = status.and_then(|s| s.get("restartCount")).and_then(|r| r.as_u64());
    let ready = status
        .and_then(|s| s.get("ready"))
        .and_then(|r| r.as_bool())
        .unwrap_or_default();

    let completed = status
        .and_then(|s| s.get("state"))
        .and_then(|s| s.get("terminated"))
        .and_then(|w| w.get("reason"))
        .and_then(|r| r.as_str());
    let is_running = status.and_then(|s| s.get("state")).and_then(|s| s.get("running")).is_some();
    let phase = if is_running {
        "Running"
    } else if let Some(completed) = completed {
        completed
    } else {
        status
            .and_then(|s| s.get("state"))
            .and_then(|s| s.get("waiting"))
            .and_then(|w| w.get("reason"))
            .and_then(|r| r.as_str())
            .unwrap_or("Unknown")
    };

    let values: [ResourceValue; 5] = [
        ResourceValue::numeric(restarts.map(|r| r.to_string()), 5),
        ready.into(),
        phase.into(),
        is_init_container.into(),
        image.into(),
    ];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: completed.is_some(),
        is_ready: is_running,
        is_terminating,
    }
}

/// Returns [`Header`] for the pod's `container`.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::fixed("RESTARTS", 3, true),
            Column::fixed("READY", 7, false),
            Column::bound("STATE", 10, 20, false),
            Column::fixed("INIT", 6, false),
            Column::bound("IMAGE", 15, 70, false),
        ])),
        Rc::new([' ', 'N', 'R', 'E', 'S', 'T', 'I', 'A']),
    )
}
