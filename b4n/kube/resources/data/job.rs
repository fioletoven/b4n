use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::jiff::Timestamp;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `job` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let succeeded = status["succeeded"].as_u64().unwrap_or_default();
    let completions = object.data["spec"]["completions"].as_u64().unwrap_or_default();
    let ctime: Option<Timestamp> = status["completionTime"].as_str().and_then(|t| t.parse().ok());
    let stime: Option<Timestamp> = status["startTime"].as_str().and_then(|t| t.parse().ok());
    let duration = ctime.and_then(|c| stime.map(|s| Timestamp::now() - (c - s)));
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [
        format!("{succeeded}/{completions}").into(),
        ResourceValue::from(duration.as_ref()),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `job` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("COMPLETIONS", 7, true),
            Column::fixed("DURATION", 9, true),
        ])),
        Rc::new([' ', 'N', 'C', 'D', 'A']),
    )
}
