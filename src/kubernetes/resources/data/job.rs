use k8s_openapi::chrono::{DateTime, Utc};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `job` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let succeeded = status["succeeded"].as_u64().unwrap_or_default();
    let completions = object.data["spec"]["completions"].as_u64().unwrap_or_default();
    let ctime: Option<DateTime<Utc>> = status["completionTime"].as_str().and_then(|t| t.parse().ok());
    let stime: Option<DateTime<Utc>> = status["startTime"].as_str().and_then(|t| t.parse().ok());
    let duration = ctime.and_then(|c| stime.map(|s| Utc::now() - (c - s)));
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [
        format!("{succeeded}/{completions}").into(),
        ResourceValue::datetime(duration.as_ref()),
    ];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the `job` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::fixed("COMPLETIONS", 7, true),
            Column::fixed("DURATION", 9, true),
        ])),
        Rc::new([' ', 'N', 'C', 'D', 'A']),
    )
}
