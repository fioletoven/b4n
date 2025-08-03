use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `namespace` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let status = &object.data["status"];
    let phase = status["phase"].as_str().map(ToOwned::to_owned);
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 1] = [if is_terminating { "Terminating".into() } else { phase.into() }];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `namespace` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([Column::bound("STATUS", 10, 20, false)])),
        Rc::new([' ', 'N', 'S', 'A']),
    )
}
