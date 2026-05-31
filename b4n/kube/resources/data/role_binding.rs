use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `rolebinding` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 3] = [
        object.data["roleRef"]["name"].as_str().into(),
        get_subject_kinds(object.data["subjects"].as_array()).into(),
        get_subject_names(object.data["subjects"].as_array()).into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `rolebinding` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("ROLE", 6, 60, false),
            Column::bound("KINDS", 6, 30, false),
            Column::bound("SUBJECTS", 10, 60, false),
        ])),
        Rc::new([' ', 'N', 'R', 'K', 'S', 'A']),
    )
}

fn get_subject_kinds(subjects: Option<&Vec<Value>>) -> Option<String> {
    Some(
        subjects?
            .iter()
            .filter_map(|subject| subject["kind"].as_str())
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn get_subject_names(subjects: Option<&Vec<Value>>) -> Option<String> {
    Some(subjects?.iter().filter_map(get_subject_name).collect::<Vec<_>>().join(","))
}

fn get_subject_name(subject: &Value) -> Option<String> {
    let name = subject["name"].as_str()?;
    match (subject["kind"].as_str(), subject["namespace"].as_str()) {
        (Some("ServiceAccount"), Some(namespace)) => Some(format!("{namespace}/{name}")),
        _ => Some(name.to_owned()),
    }
}
