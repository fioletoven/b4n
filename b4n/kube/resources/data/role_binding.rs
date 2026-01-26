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
        get_subjects(object.data["subjects"].as_array(), "kind").into(),
        get_subjects(object.data["subjects"].as_array(), "name").into(),
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

fn get_subjects(subjects: Option<&Vec<Value>>, key: &str) -> Option<String> {
    Some(subjects?.iter().filter_map(|s| s[key].as_str()).collect::<Vec<_>>().join(","))
}
