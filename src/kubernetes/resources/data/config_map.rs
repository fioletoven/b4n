use k8s_openapi::serde_json::Map;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `configmap` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let data_count = object.data["data"].as_object().map_or(0, Map::len);
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 1] = [ResourceValue::integer(Some(data_count as i64), 5)];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the `configmap` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([Column::fixed("DATA", 5, true)])),
        Rc::new([' ', 'N', 'D', 'A']),
    )
}
