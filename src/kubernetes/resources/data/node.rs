use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::{collections::BTreeMap, rc::Rc};

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
    let taints = i64::try_from(object.data["spec"]["taints"].as_array().map(|t| t.len()).unwrap_or_default()).ok();
    let version = status["nodeInfo"]["kubeletVersion"].as_str();
    let name = object.metadata.name.as_deref().unwrap_or_default();
    let pods = i64::try_from(stats.pods_count(name)).ok();
    let containers = i64::try_from(stats.containers_count(name)).ok();
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 6] = [
        ResourceValue::integer(taints, 3),
        get_first_status(status["conditions"].as_array()).into(),
        get_roles(object.metadata.labels.as_ref()).into(),
        version.into(),
        ResourceValue::integer(pods, 6),
        ResourceValue::integer(containers, 6),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `nodes` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("TAINTS", 2, true),
            Column::bound("STATUS", 8, 25, false),
            Column::bound("ROLE", 6, 30, false),
            Column::bound("VERSION", 15, 30, false),
            Column::fixed("PODS", 5, true),
            Column::fixed("CONTAINERS", 5, true),
        ])),
        Rc::new([' ', 'N', 'T', 'S', 'R', 'V', 'P', 'C', 'A']),
    )
}

fn get_first_status(conditions: Option<&Vec<Value>>) -> Option<&str> {
    conditions?
        .iter()
        .find(|c| c["status"].as_str() == Some("True"))
        .and_then(|c| c["type"].as_str())
}

fn get_roles(labels: Option<&BTreeMap<String, String>>) -> Option<String> {
    labels.map(|labels| {
        labels
            .iter()
            .filter(|(l, v)| l.starts_with("node-role.kubernetes.io/") && *v == "true")
            .map(|(l, _)| &l[24..])
            .collect::<Vec<_>>()
            .join(",")
    })
}
