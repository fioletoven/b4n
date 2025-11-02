use b4n_tui::grid::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Map;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kubernetes::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `configmap` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let data_count = object.data["data"].as_object().map_or(0, Map::len);
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 1] = [ResourceValue::integer(i64::try_from(data_count).ok(), 5)];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `configmap` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([Column::fixed("DATA", 5, true)])),
        Rc::new([' ', 'N', 'D', 'A']),
    )
}
