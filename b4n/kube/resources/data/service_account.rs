use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `serviceaccount` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let secrets = object.data["secrets"]
        .as_array()
        .map(|s| i64::try_from(s.len()).unwrap_or_default());
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let values: [ResourceValue; 1] = [ResourceValue::integer(secrets, 7)];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `serviceaccount` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([Column::fixed("SECRETS", 8, true)])),
        Rc::new([' ', 'N', 'S', 'A']),
    )
}
