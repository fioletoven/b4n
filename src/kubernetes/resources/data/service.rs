use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `service` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [spec["type"].as_str().into(), spec["clusterIP"].as_str().into()];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `service` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("TYPE", 8, 12, false),
            Column::bound("CLUSTER-IP", 11, 16, false),
        ])),
        Rc::new([' ', 'N', 'T', 'C', 'A']),
    )
}
