use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::{collections::HashSet, rc::Rc};

use crate::{
    kubernetes::resources::{CrdColumns, ResourceData, ResourceValue},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the custom resource.
pub fn data(crd: &CrdColumns, object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let extra_values = if crd.has_metadata_pointer {
        serialize_and_get_data(crd, object)
    } else {
        get_data(crd, object)
    };

    ResourceData {
        extra_values,
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the custom resource.
pub fn header(crd: &CrdColumns) -> Header {
    let columns = if let Some(columns) = &crd.columns {
        columns.iter().map(Column::from).collect()
    } else {
        Vec::new()
    };
    let symbols = get_sort_symbols(&columns);

    Header::from(
        NAMESPACE,
        Some(columns.into_boxed_slice()),
        Rc::from(symbols.into_boxed_slice()),
    )
}

fn get_data(crd: &CrdColumns, object: &DynamicObject) -> Box<[ResourceValue]> {
    // TODO: fix ResourceValue::from("")
    let mut data = Vec::with_capacity(crd.columns.as_ref().map(|c| c.len()).unwrap_or_default());
    if let Some(columns) = &crd.columns {
        for column in columns {
            if let Some(value) = object.data.pointer(&column.pointer) {
                data.push(get_resource_value(value, &column.field_type));
            } else {
                data.push(ResourceValue::from(" "));
            }
        }
    }

    data.into_boxed_slice()
}

fn serialize_and_get_data(crd: &CrdColumns, object: &DynamicObject) -> Box<[ResourceValue]> {
    // TODO: serialize DynamicObject to include metadata part as json Value and then applly pointer on it
    Box::default()
}

fn get_resource_value(value: &Value, field_type: &str) -> ResourceValue {
    // TODO: fix negative integer and number behaviour
    match field_type {
        "boolean" => ResourceValue::from(value.as_bool().unwrap_or_default()),
        "integer" => ResourceValue::numeric(value.as_i64().map(|i| i.to_string()), 10),
        "number" => ResourceValue::numeric(value.as_f64().map(|i| i.to_string()), 10),
        "string" => ResourceValue::from(value.as_str()),
        "date" => ResourceValue::from(value.as_str()),
        _ => ResourceValue::from(" "),
    }
}

fn get_sort_symbols(columns: &Vec<Column>) -> Vec<char> {
    let mut already_taken = HashSet::with_capacity(columns.len() + 2);
    already_taken.insert('N');
    already_taken.insert('A');

    let mut symbols = Vec::with_capacity(columns.len() + 3);
    symbols.push(' ');
    symbols.push('N');

    for column in columns {
        let mut found = false;
        for ch in column.name.chars() {
            if !already_taken.contains(&ch) {
                symbols.push(ch);
                already_taken.insert(ch);

                found = true;
                break;
            }
        }

        if !found {
            symbols.push(' ');
        }
    }

    symbols.push('A');
    symbols
}
