use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    app::lists::{Column, Header, NAMESPACE},
    kubernetes::resources::{ResourceData, ResourceValue},
};

/// Returns [`ResourceData`] for the `secret` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let secret_type = object.data["type"].as_str().map(|t| t.to_owned());
    let data_count = object.data["data"].as_object().map(|o| o.len()).unwrap_or(0).to_string();
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [secret_type.into(), ResourceValue::numeric(Some(data_count), 5)];

    ResourceData {
        extra_values: Box::new(values),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the `secret` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE.clone(),
        Some(Box::new([
            Column::bound("TYPE", 8, 25, false),
            Column::fixed("DATA", 5, true),
        ])),
        Rc::new([' ', 'N', 'T', 'D', 'A']),
    )
}
