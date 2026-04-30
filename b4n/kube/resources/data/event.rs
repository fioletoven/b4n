use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::{ResourceExt, api::DynamicObject};
use std::rc::Rc;

use crate::kube::resources::{ColumnsLayout, ResourceData, ResourceValue};

/// Returns name for the `event` kubernetes resource.
pub fn name(object: &DynamicObject, columns_layout: ColumnsLayout) -> String {
    match columns_layout {
        ColumnsLayout::General => object.name_any(),
        ColumnsLayout::Individual => object.name_any(),
        ColumnsLayout::Compact => object.data["type"].as_str().map(String::from).unwrap_or_default(),
    }
}

/// Returns [`ResourceData`] for the `event` kubernetes resource.
pub fn data(object: &DynamicObject, columns_layout: ColumnsLayout) -> ResourceData {
    match columns_layout {
        ColumnsLayout::General => data_general(object),
        ColumnsLayout::Individual => data_individual(object),
        ColumnsLayout::Compact => data_compact(object),
    }
}

/// Returns [`Header`] for the `event` kubernetes resource.
pub fn header(columns_layout: ColumnsLayout) -> Header {
    match columns_layout {
        ColumnsLayout::General => header_general(),
        ColumnsLayout::Individual => header_individual(),
        ColumnsLayout::Compact => header_compact(),
    }
}

fn data_general(object: &DynamicObject) -> ResourceData {
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
        event_count(object),
        object.data["type"].as_str().into(),
        object.data["reason"].as_str().into(),
        obj.into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

pub fn header_general() -> Header {
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

fn data_individual(object: &DynamicObject) -> ResourceData {
    ResourceData::new(
        Box::new([
            event_count(object),
            object.data["type"].as_str().into(),
            object.data["message"].as_str().into(),
        ]),
        object.metadata.deletion_timestamp.is_some(),
    )
}

pub fn header_individual() -> Header {
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
    .with_stretch_last()
}

fn data_compact(object: &DynamicObject) -> ResourceData {
    ResourceData::new(
        Box::new([event_count(object), object.data["message"].as_str().into()]),
        object.metadata.deletion_timestamp.is_some(),
    )
}

pub fn header_compact() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("COUNT", 6, true),
            Column::bound("MESSAGE", 15, 150, false),
        ])),
        Rc::new([' ', 'T', 'C', 'M', 'A']),
    )
    .with_name_column(Column::bound("TYPE", 6, 6, false))
    .with_sort_info(4, false)
    .with_stretch_last()
}

fn event_count(object: &DynamicObject) -> ResourceValue {
    if let Some(count) = object.data["count"].as_i64() {
        ResourceValue::integer(Some(count), 6)
    } else {
        ResourceValue::integer(object.data["series"]["count"].as_i64(), 6)
    }
}
