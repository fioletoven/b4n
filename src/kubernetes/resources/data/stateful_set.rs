use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `statefulset` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let replicas = status["replicas"].as_u64().unwrap_or_default();
    let ready = status["readyReplicas"].as_u64().unwrap_or_default();
    let updated = status["updatedReplicas"].as_u64().unwrap_or_default();
    let available = status["availableReplicas"].as_u64().unwrap_or_default();
    let service = object.data["spec"]["serviceName"].as_str().map(String::from);
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 4] = [
        format!("{ready}/{replicas}").into(),
        format!("{updated}/{replicas}").into(),
        format!("{available}/{replicas}").into(),
        service.into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `statefulset` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("READY", 6, true),
            Column::fixed("UPDATED", 8, true),
            Column::fixed("AVAILABLE", 10, true),
            Column::bound("SERVICE", 8, 30, false),
        ])),
        Rc::new([' ', 'N', 'R', 'U', 'V', 'S', 'A']),
    )
}
