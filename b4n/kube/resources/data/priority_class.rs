use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `priorityclass` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 2] = [
        ResourceValue::integer(object.data["value"].as_i64(), 12),
        object.data["globalDefault"].as_bool().unwrap_or_default().into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `priorityclass` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("VALUE", 12, true),
            Column::fixed("GLOBAL DEFAULT", 15, false),
        ])),
        Rc::new([' ', 'N', 'V', 'G', 'A']),
    )
}
