use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::{
        resources::{ResourceData, ResourceValue},
        watchers::Statistics,
    },
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `nodes` kubernetes resource.
pub fn data(object: &DynamicObject, stats: &Statistics) -> ResourceData {
    let status = &object.data["status"];
    let name = object.metadata.name.as_deref().unwrap_or_default();
    let version = status["nodeInfo"]["kubeletVersion"].as_str();
    let pods = i64::try_from(stats.pods_count(name)).unwrap_or_default();
    let containers = i64::try_from(stats.containers_count(name)).unwrap_or_default();
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 4] = [
        get_first_status(status["conditions"].as_array()).into(),
        version.into(),
        ResourceValue::integer(Some(pods), 6),
        ResourceValue::integer(Some(containers), 6),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `nodes` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("STATUS", 8, 25, false),
            Column::bound("VERSION", 15, 30, false),
            Column::fixed("PODS", 5, true),
            Column::fixed("CONTAINERS", 5, true),
        ])),
        Rc::new([' ', 'N', 'S', 'V', 'P', 'C', 'A']),
    )
}

fn get_first_status(conditions: Option<&Vec<Value>>) -> Option<&str> {
    if let Some(conditions) = conditions {
        conditions
            .iter()
            .find(|c| c["status"].as_str().is_some_and(|s| s == "True"))
            .and_then(|s| s["type"].as_str())
    } else {
        None
    }
}
