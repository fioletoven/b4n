use b4n_kube::stats::MemoryMetrics;
use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::{rc::Rc, str::FromStr};

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `persistentvolume` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let phase = object.data["status"]["phase"].as_str();
    let capacity = spec["capacity"]["storage"]
        .as_str()
        .and_then(|m| MemoryMetrics::from_str(m).ok());
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let is_ready = phase.is_some_and(|p| p == "Bound");

    let values: [ResourceValue; 6] = [
        phase.into(),
        capacity.into(),
        get_access_modes(spec["accessModes"].as_array()).into(),
        spec["persistentVolumeReclaimPolicy"].as_str().into(),
        get_claim(&spec["claimRef"]).into(),
        spec["storageClassName"].as_str().into(),
    ];

    ResourceData {
        extra_values: Box::new(values),
        is_ready: !is_terminating && is_ready,
        is_terminating,
        ..Default::default()
    }
}

/// Returns [`Header`] for the `persistentvolume` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("STATUS", 6, 15, false),
            Column::bound("CAPACITY", 8, 15, false),
            Column::bound("ACCESS MODES", 12, 20, false),
            Column::bound("RECLAIM POLICY", 14, 20, false),
            Column::bound("CLAIM", 15, 45, false),
            Column::bound("STORAGE CLASS", 12, 30, false),
        ])),
        Rc::new([' ', 'N', 'S', 'C', 'M', 'R', 'L', 'T', 'A']),
    )
}

/// Returns access modes from the JSON value.
pub fn get_access_modes(access_modes: Option<&Vec<Value>>) -> String {
    if let Some(modes) = access_modes {
        modes
            .iter()
            .filter_map(|m| m.as_str())
            .filter_map(|m| match m {
                "ReadWriteOnce" => Some("RWO"),
                "ReadOnlyMany" => Some("ROX"),
                "ReadWriteMany" => Some("RWX"),
                "ReadWriteOncePod" => Some("RWOP"),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(",")
    } else {
        String::default()
    }
}

fn get_claim(claim_ref: &Value) -> String {
    let name = claim_ref["name"].as_str().unwrap_or_default();
    let namespace = claim_ref["namespace"].as_str();
    match namespace {
        Some(ns) => format!("{ns}/{name}"),
        None => name.to_owned(),
    }
}
