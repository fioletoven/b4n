use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::{
        resources::{ResourceData, ResourceValue},
        utils::labels_to_string,
    },
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `service` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let selector = spec["selector"].as_object().map(labels_to_string);
    let tags = if let Some(selector) = selector {
        Box::new([selector])
    } else {
        Box::default()
    };

    let values: [ResourceValue; 2] = [spec["type"].as_str().into(), spec["clusterIP"].as_str().into()];

    ResourceData::new(Box::new(values), is_terminating).with_tags(tags)
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
