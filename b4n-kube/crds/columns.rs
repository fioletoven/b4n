use k8s_openapi::serde_json::Value;
use std::{cell::RefCell, rc::Rc};

const DEFAULT_PATHS: [&str; 3] = [".metadata.name", ".metadata.namespace", ".metadata.creationTimestamp"];

pub type SharedCrdsList = Rc<RefCell<Vec<CrdColumns>>>;

/// Holds data about custom columns defined in CRD resource.
#[derive(Debug, Clone)]
pub struct CrdColumns {
    pub uid: String,
    pub name: String,
    pub columns: Option<Vec<CrdColumn>>,
    pub has_metadata_pointer: bool,
}

impl CrdColumns {
    /// Creates new [`CrdColumns`] instance from [`DynamicObject`] resource.\
    /// **Note** that it skips default columns that will be shown anyway.
    pub fn from(uid: &str, kind: &str, version: &Value) -> Self {
        let name = version["name"].as_str().unwrap_or_default();
        let columns = version["additionalPrinterColumns"].as_array().map(|c| {
            c.iter()
                .filter(|c| !is_default(c))
                .map(CrdColumn::from)
                .filter(|c| c.priority == 0)
                .collect::<Vec<_>>()
        });

        let has_metadata_pointer = columns
            .as_ref()
            .is_some_and(|c| c.iter().any(|c| c.pointer.starts_with("/metadata")));

        Self {
            uid: format!("{uid}.{name}"),
            name: format!("{kind}/{name}"),
            columns,
            has_metadata_pointer,
        }
    }
}

/// Contains CRD's custom column data.
#[derive(Debug, Clone)]
pub struct CrdColumn {
    pub name: String,
    pub pointer: String,
    pub field_type: String,
    pub priority: i64,
}

impl CrdColumn {
    /// Creates new [`CrdColumn`] instance from the json [`Value`].
    pub fn from(value: &Value) -> Self {
        Self {
            name: get_string(value, "name"),
            pointer: to_json_pointer(get_str(value, "jsonPath")),
            field_type: get_string(value, "type"),
            priority: get_integer(value, "priority"),
        }
    }
}

fn get_integer(value: &Value, field_name: &str) -> i64 {
    value.get(field_name).and_then(Value::as_i64).unwrap_or_default()
}

fn get_string(value: &Value, field_name: &str) -> String {
    value
        .get(field_name)
        .and_then(|n| n.as_str())
        .map(String::from)
        .unwrap_or_default()
}

fn get_str<'a>(value: &'a Value, field_name: &str) -> &'a str {
    value.get(field_name).and_then(|n| n.as_str()).unwrap_or_default()
}

fn is_default(column: &Value) -> bool {
    column
        .get("jsonPath")
        .is_some_and(|p| p.as_str().is_some_and(|s| DEFAULT_PATHS.contains(&s)))
}

fn to_json_pointer(jsonpath: &str) -> String {
    let mut result = String::with_capacity(jsonpath.len());

    for ch in jsonpath.chars() {
        if ch == '.' || ch == '[' {
            result.push('/');
        } else if ch == '~' {
            result.push('~');
            result.push('0');
        } else if ch == '/' {
            result.push('~');
            result.push('1');
        } else if ch != ']' && ch != '$' {
            result.push(ch);
        }
    }

    result
}
