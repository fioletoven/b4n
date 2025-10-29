use k8s_openapi::serde_json::Map;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `secret` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let secret_type = object.data["type"].as_str();
    let data_count = object.data["data"].as_object().map_or(0, Map::len);
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [secret_type.into(), ResourceValue::integer(Some(data_count as i64), 5)];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `secret` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("TYPE", 8, 25, false),
            Column::fixed("DATA", 5, true),
        ])),
        Rc::new([' ', 'N', 'T', 'D', 'A']),
    )
}
