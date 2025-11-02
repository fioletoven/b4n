use b4n_kube::utils::get_match_labels;
use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kubernetes::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `deployment` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let replicas = status["replicas"].as_u64().unwrap_or_default();
    let ready = status["readyReplicas"].as_u64().unwrap_or_default();
    let updated = status["updatedReplicas"].as_u64().unwrap_or_default();
    let available = status["availableReplicas"].as_u64().unwrap_or_default();
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let tags = get_match_labels(object);

    let values: [ResourceValue; 3] = [
        format!("{ready}/{replicas}").into(),
        format!("{updated}/{replicas}").into(),
        format!("{available}/{replicas}").into(),
    ];

    ResourceData::new(Box::new(values), is_terminating).with_tags(tags)
}

/// Returns [`Header`] for the `deployment` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("READY", 6, true),
            Column::fixed("UPDATED", 8, true),
            Column::fixed("AVAILABLE", 10, true),
        ])),
        Rc::new([' ', 'N', 'R', 'U', 'V', 'A']),
    )
}
