use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `networkpolicy` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let values: [ResourceValue; 1] = [get_selector(object.data["spec"]["podSelector"]["matchLabels"].as_object()).into()];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `networkpolicy` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([Column::bound("POD SELECTOR", 15, 50, false)])),
        Rc::new([' ', 'N', 'P', 'A']),
    )
}

fn get_selector(labels: Option<&Map<String, Value>>) -> Option<String> {
    let labels = labels?
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|v| format!("{k}={v}")))
        .collect::<Vec<_>>();

    Some(labels.join(","))
}
