use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    app::lists::{Column, Header, NAMESPACE},
    kubernetes::resources::{ResourceData, ResourceValue},
};

/// Returns [`ResourceData`] for the `service` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let service_type = spec["type"].as_str().map(|t| t.to_owned());
    let cluster_ip = spec["clusterIP"].as_str().map(|t| t.to_owned());
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [service_type.into(), cluster_ip.into()];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the `service` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::bound("TYPE", 8, 12, false),
            Column::bound("CLUSTER-IP", 11, 16, false),
        ])),
        Rc::new([' ', 'N', 'T', 'C', 'A']),
    )
}
