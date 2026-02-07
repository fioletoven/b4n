use b4n_kube::stats::{CpuMetrics, MemoryMetrics, Statistics};
use b4n_list::Item;
use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::{rc::Rc, slice::IterMut};

use crate::kube::resources::{ResourceData, ResourceFilterContext, ResourceItem, ResourceValue};

const COLUMNS_NO_WITH_STATS: usize = 7;

/// Returns [`ResourceData`] for the `pod` kubernetes resource.
pub fn data(object: &DynamicObject, statistics: &Statistics) -> ResourceData {
    let status = &object.data["status"];
    let spec = &object.data["spec"];
    let ready = status["containerStatuses"].as_array().map(|c| get_ready(c));
    let phase = status["phase"].as_str();
    let waiting = status["containerStatuses"]
        .as_array()
        .and_then(|c| get_first_waiting_reason(c));
    let restarts = status["containerStatuses"].as_array().map(|c| get_restarts(c));
    let node = spec["nodeName"].as_str();
    let is_completed = if let Some(ph) = &phase { *ph == "Succeeded" } else { false };
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let (ready_str, is_ready) = if let Some(ready) = ready {
        (Some(ready.0), ready.1)
    } else {
        (None, false)
    };

    let mut values = vec![
        ResourceValue::integer(restarts, 5),
        ready_str.into(),
        if is_terminating {
            "Terminating".into()
        } else if waiting.is_some() {
            waiting.into()
        } else {
            phase.into()
        },
    ];

    if statistics.has_metrics {
        if let Some(node_name) = node
            && let Some(pod_name) = object.metadata.name.as_deref()
            && let Some(pod_namespace) = object.metadata.namespace.as_deref()
            && let Some(stats) = statistics.pod(node_name, pod_name, pod_namespace)
        {
            values.push(stats.metrics.map(|m| m.cpu).into());
            values.push(stats.metrics.map(|m| m.memory).into());
        } else {
            values.push(None::<CpuMetrics>.into());
            values.push(None::<MemoryMetrics>.into());
        }
    }

    values.push(status["podIP"].as_str().into());
    values.push(node.into());

    ResourceData {
        extra_values: values.into_boxed_slice(),
        is_completed,
        is_ready: !is_terminating && is_ready,
        is_terminating,
        tags: get_single_container(&spec["containers"]),
    }
}

/// Returns [`Header`] for the `pod` kubernetes resource.
pub fn header(has_metrics: bool) -> Header {
    let mut columns = vec![
        Column::fixed("RESTARTS", 3, true),
        Column::fixed("READY", 7, false),
        Column::bound("STATUS", 10, 20, false),
    ];

    let mut symbols = vec![' ', 'N', 'R', 'E', 'S'];

    if has_metrics {
        columns.push(Column::bound("CPU", 5, 15, true));
        columns.push(Column::bound("MEM", 5, 15, true));
        symbols.push('C');
        symbols.push('M');
    }

    columns.push(Column::bound("IP", 11, 16, false));
    columns.push(Column::bound("NODE", 12, 25, false));

    symbols.push('I');
    symbols.push('O');
    symbols.push('A');

    Header::from(
        NAMESPACE,
        Some(columns.into_boxed_slice()),
        Rc::from(symbols.into_boxed_slice()),
    )
}

/// Updates statistics for specified [`ResourceItem`] list mutable iterator.
pub fn update_statistics(items: IterMut<'_, Item<ResourceItem, ResourceFilterContext>>, statistics: &Statistics) {
    if !statistics.has_metrics {
        return;
    }

    for item in items {
        if let Some(data) = &mut item.data.data
            && data.extra_values.len() == COLUMNS_NO_WITH_STATS
            && let Some(node_name) = data.extra_values[6].raw_text()
            && let Some(pod_namespace) = item.data.namespace.as_deref()
            && let Some(stats) = statistics.pod(node_name, &item.data.name, pod_namespace)
        {
            data.extra_values[3] = stats.metrics.map(|m| m.cpu).into();
            data.extra_values[4] = stats.metrics.map(|m| m.memory).into();
        }
    }
}

fn get_restarts(containers: &[Value]) -> i64 {
    containers
        .iter()
        .map(|c| c["restartCount"].as_i64().unwrap_or(0))
        .sum::<i64>()
}

fn get_ready(containers: &[Value]) -> (String, bool) {
    let ready = containers.iter().filter(|c| c["ready"].as_bool().unwrap_or_default()).count();

    (format!("{}/{}", ready, containers.len()), ready == containers.len())
}

fn get_first_waiting_reason(containers: &[Value]) -> Option<String> {
    for c in containers {
        if let Some(reason) = c
            .get("state")
            .and_then(|s| s.get("waiting"))
            .and_then(|w| w.get("reason"))
            .and_then(|r| r.as_str())
        {
            return Some(reason.to_owned());
        }
    }

    None
}

fn get_single_container(containers: &Value) -> Box<[String]> {
    if let Some(name) = get_single_container_name(containers) {
        Box::new([name])
    } else {
        Box::default()
    }
}

fn get_single_container_name(containers: &Value) -> Option<String> {
    match containers.as_array()?.as_slice() {
        [one] => one.as_object()?.get("name")?.as_str().map(String::from),
        _ => None,
    }
}
