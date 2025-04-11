use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `daemonset` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let desired = status["desiredNumberScheduled"].as_u64().unwrap_or_default();
    let current = status["currentNumberScheduled"].as_u64().unwrap_or_default();
    let ready = status["numberReady"].as_u64().unwrap_or_default();
    let updated = status["updatedNumberScheduled"].as_u64().unwrap_or_default();
    let available = status["numberAvailable"].as_u64().unwrap_or_default();
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 5] = [
        ResourceValue::numeric(Some(desired.to_string()), 5),
        ResourceValue::numeric(Some(current.to_string()), 5),
        ResourceValue::numeric(Some(ready.to_string()), 5),
        ResourceValue::numeric(Some(updated.to_string()), 5),
        ResourceValue::numeric(Some(available.to_string()), 5),
    ];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the `daemonset` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::fixed("DESIRED", 3, true),
            Column::fixed("CURRENT", 8, true),
            Column::fixed("READY", 6, true),
            Column::fixed("UPDATED", 8, true),
            Column::fixed("AVAILABLE", 10, true),
        ])),
        Rc::new([' ', 'N', 'D', 'C', 'R', 'U', 'V', 'A']),
    )
}
