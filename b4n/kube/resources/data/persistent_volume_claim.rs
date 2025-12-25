use b4n_kube::stats::MemoryMetrics;
use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::{rc::Rc, str::FromStr};

use crate::kube::resources::{ResourceData, ResourceValue, persistent_volume};

/// Returns [`ResourceData`] for the `persistentvolumeclaim` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let phase = object.data["status"]["phase"].as_str();
    let capacity = spec["resources"]["requests"]["storage"]
        .as_str()
        .and_then(|m| MemoryMetrics::from_str(m).ok());
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let is_ready = phase.is_some_and(|p| p == "Bound");

    let values: [ResourceValue; 5] = [
        phase.into(),
        spec["volumeName"].as_str().into(),
        capacity.into(),
        persistent_volume::get_access_modes(spec["accessModes"].as_array()).into(),
        spec["storageClassName"].as_str().into(),
    ];

    ResourceData {
        extra_values: Box::new(values),
        is_ready: !is_terminating && is_ready,
        is_terminating,
        ..Default::default()
    }
}

/// Returns [`Header`] for the `persistentvolumeclaim` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("STATUS", 6, 15, false),
            Column::bound("VOLUME", 15, 50, false),
            Column::bound("CAPACITY", 8, 15, false),
            Column::bound("ACCESS MODES", 12, 20, false),
            Column::bound("STORAGE CLASS", 12, 30, false),
        ])),
        Rc::new([' ', 'N', 'S', 'V', 'C', 'M', 'T', 'A']),
    )
}
