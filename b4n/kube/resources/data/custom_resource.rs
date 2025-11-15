use b4n_kube::crds::CrdColumns;
use b4n_tui::grid::{Column, Header, NAMESPACE};
use jsonpath_rust::JsonPath;
use k8s_openapi::serde_json::{Value, to_value};
use kube::api::DynamicObject;
use std::{collections::HashSet, rc::Rc};

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the custom resource.
pub fn data(crd: &CrdColumns, object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let extra_values = if crd.has_metadata_pointer {
        // we need to serialize DynamicObject as metadata part is not directly accessible using pointer method
        if let Ok(value) = to_value(object) {
            get_data(crd, &value)
        } else {
            Box::default()
        }
    } else {
        get_data(crd, &object.data)
    };

    ResourceData::new(extra_values, is_terminating)
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

fn get_data(crd: &CrdColumns, object_data: &Value) -> Box<[ResourceValue]> {
    let mut data = Vec::with_capacity(crd.columns.as_ref().map(Vec::len).unwrap_or_default());
    if let Some(columns) = &crd.columns {
        for column in columns {
            if let Ok(value) = object_data.query(&column.json_path)
                && !value.is_empty()
            {
                data.push(get_resource_value(value[0], &column.field_type));
            } else {
                data.push(ResourceValue::from(""));
            }
        }
    }

    data.into_boxed_slice()
}

fn get_resource_value(value: &Value, field_type: &str) -> ResourceValue {
    match field_type {
        "boolean" => ResourceValue::from(value.as_bool().unwrap_or_default()),
        "integer" => ResourceValue::integer(value.as_i64(), 10),
        "number" => ResourceValue::number(value.as_f64(), 10),
        "string" => ResourceValue::from(value.as_str()),
        "date" => ResourceValue::time(value.clone()),
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
