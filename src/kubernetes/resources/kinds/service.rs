use kube::api::DynamicObject;

use crate::{
    app::lists::{Column, Header, NAMESPACE},
    kubernetes::resources::ResourceData,
};

/// Returns [`ResourceData`] for the `service` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let service_type = spec["type"].as_str().map(|t| t.to_owned());
    let cluster_ip = spec["clusterIP"].as_str().map(|t| t.to_owned());

    let values = [service_type, cluster_ip];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: false,
        is_ready: true,
        is_terminating: false,
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
    )
}
