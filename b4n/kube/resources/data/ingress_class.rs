use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{kube::resources::ResourceData, ui::widgets::table::Cell};

const DEFAULT_CLASS_ANNOTATION: &str = "ingressclass.kubernetes.io/is-default-class";

/// Returns [`ResourceData`] for the `ingressclass` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let parameters = &object.data["spec"]["parameters"];
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let is_default = object
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(DEFAULT_CLASS_ANNOTATION))
        .is_some_and(|value| value == "true");

    let values: [Cell; 3] = [
        object.data["spec"]["controller"].as_str().into(),
        get_parameters(parameters).into(),
        is_default.into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `ingressclass` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("CONTROLLER", 12, 40, false),
            Column::bound("PARAMETERS", 12, 50, false),
            Column::fixed("DEFAULT", 8, false),
        ])),
        Rc::new([' ', 'N', 'C', 'P', 'D', 'A']),
    )
}

fn get_parameters(parameters: &k8s_openapi::serde_json::Value) -> Option<String> {
    let name = parameters["name"].as_str()?;
    let kind = parameters["kind"].as_str().unwrap_or_default();
    if let Some(namespace) = parameters["namespace"].as_str() {
        Some(format!("{kind}/{namespace}/{name}"))
    } else {
        Some(format!("{kind}/{name}"))
    }
}
