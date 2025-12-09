use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `apiservice` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let service = &object.data["spec"]["service"];
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [
        get_service(service["name"].as_str(), service["namespace"].as_str()).into(),
        get_available_condition(object.data["status"]["conditions"].as_array()).into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `apiservice` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("SERVICE", 8, 40, false),
            Column::fixed("AVAILABLE", 11, false),
        ])),
        Rc::new([' ', 'N', 'S', 'V', 'A']),
    )
}

fn get_service(name: Option<&str>, namespace: Option<&str>) -> String {
    if let Some(name) = name {
        if let Some(namespace) = namespace {
            format!("{namespace}/{name}")
        } else {
            name.to_owned()
        }
    } else {
        "Local".to_owned()
    }
}

fn get_available_condition(conditions: Option<&Vec<Value>>) -> String {
    conditions
        .and_then(|c| {
            c.iter()
                .find(|c| c["type"].as_str().is_some_and(|t| t == "Available"))
                .and_then(|v| v["status"].as_str())
        })
        .unwrap_or("False")
        .to_owned()
}
