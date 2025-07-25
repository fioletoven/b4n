use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `daemonset` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let desired = status["desiredNumberScheduled"].as_i64();
    let current = status["currentNumberScheduled"].as_i64();
    let ready = status["numberReady"].as_i64();
    let updated = status["updatedNumberScheduled"].as_i64();
    let available = status["numberAvailable"].as_i64();
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 5] = [
        ResourceValue::integer(desired, 5),
        ResourceValue::integer(current, 5),
        ResourceValue::integer(ready, 5),
        ResourceValue::integer(updated, 5),
        ResourceValue::integer(available, 5),
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
        NAMESPACE,
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
