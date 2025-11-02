use b4n_kube::stats::{CpuMetrics, MemoryMetrics};
use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::{rc::Rc, str::FromStr};

use crate::kubernetes::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `podmetrics` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let containers = object.data["containers"].as_array();
    let memory = containers
        .map(|c| {
            c.iter()
                .filter_map(|c| c["usage"]["memory"].as_str())
                .filter_map(|m| MemoryMetrics::from_str(m).ok())
                .sum::<MemoryMetrics>()
        })
        .unwrap_or_default();
    let cpu = containers
        .map(|c| {
            c.iter()
                .filter_map(|c| c["usage"]["cpu"].as_str())
                .filter_map(|m| CpuMetrics::from_str(m).ok())
                .sum::<CpuMetrics>()
        })
        .unwrap_or_default();

    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 3] = [
        Some(cpu.to_string()).into(),
        Some(memory.to_string()).into(),
        object.data["window"].as_str().into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `podmetrics` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("CPU", 8, 15, false),
            Column::bound("MEMORY", 8, 15, false),
            Column::bound("WINDOW", 8, 15, false),
        ])),
        Rc::new([' ', 'N', 'C', 'M', 'W', 'A']),
    )
}
