use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `event` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let last = if object.data["lastTimestamp"].is_null() {
        object.data["eventTime"].clone()
    } else {
        object.data["lastTimestamp"].clone()
    };
    let count = object.data["count"].as_i64().unwrap_or_default();
    let r#type = object.data["type"].as_str().map(ToOwned::to_owned);
    let reason = object.data["reason"].as_str().map(ToOwned::to_owned);
    let obj = &object.data["involvedObject"];
    let kind = obj["kind"].as_str().unwrap_or_default().to_ascii_lowercase();
    let name = obj["name"].as_str().unwrap_or_default();
    let obj = if !kind.is_empty() || !name.is_empty() {
        format!("{kind}/{name}")
    } else {
        "n/a".to_owned()
    };
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 5] = [
        ResourceValue::time(last),
        ResourceValue::integer(Some(count), 6),
        r#type.into(),
        reason.into(),
        obj.into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `event` kubernetes resource.
pub fn header() -> Header {
    let mut last = Column::fixed("LAST", 6, true);
    last.has_reversed_order = true;

    Header::from(
        NAMESPACE,
        Some(Box::new([
            last,
            Column::fixed("COUNT", 6, true),
            Column::bound("TYPE", 6, 7, false),
            Column::bound("REASON", 6, 25, false),
            Column::bound("OBJECT", 15, 70, false),
        ])),
        Rc::new([' ', 'N', 'L', 'C', 'T', 'R', 'O', 'A']),
    )
}
