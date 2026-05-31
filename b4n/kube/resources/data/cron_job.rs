use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `cronjob` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let status = &object.data["status"];
    let active = status["active"].as_array().map_or(0, Vec::len);
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 5] = [
        spec["schedule"].as_str().into(),
        spec["suspend"].as_bool().unwrap_or_default().into(),
        active.to_string().into(),
        ResourceValue::time(status["lastScheduleTime"].clone()),
        ResourceValue::time(status["lastSuccessfulTime"].clone()),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `cronjob` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("SCHEDULE", 14, 32, false),
            Column::fixed("SUSPEND", 8, false),
            Column::fixed("ACTIVE", 8, true),
            Column::fixed("LAST SCHEDULE", 14, true),
            Column::fixed("LAST SUCCESS", 14, true),
        ])),
        Rc::new([' ', 'N', 'S', 'U', 'C', 'L', 'T', 'A']),
    )
}
