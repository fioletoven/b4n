use b4n_tui::grid::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `customresourcedefinition` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let versions = spec["versions"]
        .as_array()
        .map(|v| v.iter().filter_map(|v| v["name"].as_str()).collect::<Vec<_>>().join(","));
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 4] = [
        spec["group"].as_str().into(),
        spec["names"]["kind"].as_str().into(),
        versions.into(),
        spec["scope"].as_str().into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `customresourcedefinition` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("GROUP", 8, 30, false),
            Column::bound("KIND", 6, 30, false),
            Column::bound("VERSION", 8, 15, false),
            Column::fixed("SCOPE", 10, false),
        ])),
        Rc::new([' ', 'N', 'G', 'K', 'V', 'S', 'A']),
    )
}
