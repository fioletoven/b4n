use b4n_kube::stats::MemoryMetrics;
use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;
use std::{rc::Rc, str::FromStr};

use crate::{kube::resources::ResourceData, ui::widgets::table::Cell};

/// Returns [`ResourceData`] for the `csistoragecapacity` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let capacity = object.data["capacity"]
        .as_str()
        .and_then(|m| MemoryMetrics::from_str(m).ok())
        .unwrap_or_default();
    let max_size = object.data["maximumVolumeSize"]
        .as_str()
        .and_then(|m| MemoryMetrics::from_str(m).ok())
        .unwrap_or_default();

    let values: [Cell; 4] = [
        Some(max_size).into(),
        Some(capacity).into(),
        object.data["storageClassName"].as_str().into(),
        get_topology(&object.data["nodeTopology"]).into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `csistoragecapacity` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("MAX VOLUME", 11, 16, true),
            Column::bound("CAPACITY", 10, 16, true),
            Column::bound("STORAGE CLASS", 13, 30, false),
            Column::bound("TOPOLOGY", 12, 60, false),
        ])),
        Rc::new([' ', 'N', 'M', 'C', 'S', 'T', 'A']),
    )
}

fn get_topology(selector: &Value) -> String {
    if selector.is_null() {
        return "none".to_owned();
    }

    let labels = selector["matchLabels"].as_object();
    let expressions = selector["matchExpressions"].as_array().map_or(0, Vec::len);
    let labels = labels_to_string(labels);

    match (labels, expressions) {
        (Some(labels), 0) => labels,
        (Some(labels), count) => format!("{labels} (+{count} exprs)"),
        (None, 0) => "all".to_owned(),
        (None, count) => format!("{count} exprs"),
    }
}

fn labels_to_string(labels: Option<&Map<String, Value>>) -> Option<String> {
    let mut labels = labels?
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|value| format!("{key}={value}")))
        .collect::<Vec<_>>();

    labels.sort_unstable();
    (!labels.is_empty()).then_some(labels.join(","))
}
