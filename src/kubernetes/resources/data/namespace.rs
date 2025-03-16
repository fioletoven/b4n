use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    app::lists::{Column, Header, NAMESPACE},
    kubernetes::resources::{ResourceData, ResourceValue},
};

/// Returns [`ResourceData`] for the `namespace` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let phase = status["phase"].as_str().map(|s| s.to_owned());
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 1] = [if is_terminating { "Terminating".into() } else { phase.into() }];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the `namespace` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([Column::bound("STATUS", 10, 20, false)])),
        Rc::new([' ', 'N', 'S', 'A']),
    )
}
