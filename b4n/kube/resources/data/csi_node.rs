use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::collections::HashSet;
use std::rc::Rc;

use crate::kube::resources::ResourceData;
use crate::ui::widgets::table::Cell;

/// Returns [`ResourceData`] for the `csinode` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let drivers = object.data["spec"]["drivers"].as_array();
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [Cell; 3] = [
        drivers.map_or(0, Vec::len).to_string().into(),
        get_topology_keys_count(drivers).to_string().into(),
        get_allocatable_limits(drivers).into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `csinode` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("DRIVERS", 8, true),
            Column::fixed("TOPOLOGY KEYS", 13, true),
            Column::bound("LIMITS", 10, 40, false),
        ])),
        Rc::new([' ', 'N', 'D', 'T', 'L', 'A']),
    )
}

fn get_topology_keys_count(drivers: Option<&Vec<Value>>) -> usize {
    drivers
        .into_iter()
        .flatten()
        .filter_map(|driver| driver["topologyKeys"].as_array())
        .flatten()
        .filter_map(|item| item.as_str())
        .collect::<HashSet<_>>()
        .len()
}

fn get_allocatable_limits(drivers: Option<&Vec<Value>>) -> Option<String> {
    let result: Vec<String> = drivers?
        .iter()
        .filter_map(|driver| {
            let name = driver["name"].as_str().unwrap_or("unknown");
            let limit = driver["allocatable"]["count"].as_i64()?;
            Some(format!("{name}={limit}"))
        })
        .collect();

    (!result.is_empty()).then(|| result.join(","))
}
