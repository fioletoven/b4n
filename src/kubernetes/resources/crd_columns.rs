use k8s_openapi::serde_json::Value;
use kube::{ResourceExt, api::DynamicObject};

const DEFAULT_COLUMNS: [&str; 3] = [".metadata.name", ".metadata.namespace", ".metadata.creationTimestamp"];

/// Holds data about custom columns defined in CRD resource.
#[derive(Debug, Clone)]
pub struct CrdColumns {
    pub uid: Option<String>,
    pub name: String,
    pub columns: Option<Vec<CrdColumn>>,
}

impl CrdColumns {
    /// Creates new [`CrdColumns`] instance from [`DynamicObject`] resource.\
    /// **Note** that it skips default columns that will be shown anyway.
    pub fn from(object: DynamicObject) -> Self {
        let columns = get_stored_version(&object)
            .and_then(|v| v.get("additionalPrinterColumns"))
            .and_then(|c| c.as_array())
            .map(|c| c.iter().filter(|c| !is_default(c)).map(CrdColumn::from).collect::<Vec<_>>());

        Self {
            uid: object.uid(),
            name: object.name_any(),
            columns,
        }
    }
}

/// Contains CRD's custom column data.
#[derive(Debug, Clone)]
pub struct CrdColumn {
    pub name: String,
    pub json_path: String,
    pub field_type: String,
}

impl CrdColumn {
    /// Creates new [`CrdColumn`] instance from the json [`Value`].
    pub fn from(value: &Value) -> Self {
        Self {
            name: get_string(value, "name"),
            json_path: get_string(value, "jsonPath"),
            field_type: get_string(value, "type"),
        }
    }
}

fn get_stored_version(object: &DynamicObject) -> Option<&Value> {
    if let Some(versions) = object
        .data
        .get("spec")
        .and_then(|s| s.get("versions"))
        .and_then(|v| v.as_array())
    {
        versions.iter().find(|v| is_stored_version(v))
    } else {
        None
    }
}

fn is_stored_version(version: &Value) -> bool {
    version.get("served").and_then(|s| s.as_bool()).unwrap_or_default()
        && version.get("storage").and_then(|s| s.as_bool()).unwrap_or_default()
}

fn get_string(value: &Value, field_name: &str) -> String {
    value
        .get(field_name)
        .and_then(|n| n.as_str())
        .map(String::from)
        .unwrap_or_default()
}

fn is_default(column: &Value) -> bool {
    column
        .get("jsonPath")
        .is_some_and(|p| p.as_str().is_some_and(|s| DEFAULT_COLUMNS.contains(&s)))
}
