use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the `event` kubernetes resource.
pub fn data(object: &DynamicObject, is_filtered: bool) -> ResourceData {
    if is_filtered {
        data_filtered(object)
    } else {
        data_full(object)
    }
}

/// Returns [`Header`] for the `event` kubernetes resource.
pub fn header(is_filtered: bool) -> Header {
    if is_filtered { header_filtered() } else { header_full() }
}

fn data_filtered(object: &DynamicObject) -> ResourceData {
    ResourceData::new(
        Box::new([
            ResourceValue::integer(object.data["count"].as_i64(), 6),
            object.data["type"].as_str().into(),
            object.data["message"].as_str().into(),
        ]),
        object.metadata.deletion_timestamp.is_some(),
    )
}

pub fn header_filtered() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("COUNT", 6, true),
            Column::bound("TYPE", 6, 7, false),
            Column::bound("MESSAGE", 15, 150, false),
        ])),
        Rc::new([' ', 'N', 'C', 'T', 'M', 'A']),
    )
    .with_sort_info(5, false)
}

fn data_full(object: &DynamicObject) -> ResourceData {
    let last = if object.data["lastTimestamp"].is_null() {
        object.data["eventTime"].clone()
    } else {
        object.data["lastTimestamp"].clone()
    };
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
        ResourceValue::integer(object.data["count"].as_i64(), 6),
        object.data["type"].as_str().into(),
        object.data["reason"].as_str().into(),
        obj.into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

pub fn header_full() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("LAST", 6, true).with_reversed_order(),
            Column::fixed("COUNT", 6, true),
            Column::bound("TYPE", 6, 7, false),
            Column::bound("REASON", 6, 25, false),
            Column::bound("OBJECT", 15, 70, false),
        ])),
        Rc::new([' ', 'N', 'L', 'C', 'T', 'R', 'O', 'A']),
    )
    .with_sort_info(2, false)
}
