use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::{collections::BTreeMap, rc::Rc, slice::IterMut};

use crate::{
    kubernetes::{
        metrics::{CpuMetrics, MemoryMetrics},
        resources::{ResourceData, ResourceFilterContext, ResourceItem, ResourceValue},
        watchers::Statistics,
    },
    ui::lists::{Column, Header, Item, NAMESPACE},
};

const COLUMNS_NO_WITH_STATS: usize = 10;

/// Returns [`ResourceData`] for the `nodes` kubernetes resource.
pub fn data(object: &DynamicObject, statistics: &Statistics) -> ResourceData {
    let status = &object.data["status"];
    let taints = i64::try_from(object.data["spec"]["taints"].as_array().map(Vec::len).unwrap_or_default()).ok();
    let version = status["nodeInfo"]["kubeletVersion"].as_str();
    let name = object.metadata.name.as_deref().unwrap_or_default();
    let pods = i64::try_from(statistics.pods_count(name)).ok();
    let containers = i64::try_from(statistics.containers_count(name)).ok();
    let ready = get_first_status(status["conditions"].as_array());
    let is_ready = ready.is_some_and(|r| r == "Ready");
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let mut values = vec![
        ResourceValue::integer(taints, 3),
        get_roles(object.metadata.labels.as_ref()).into(),
        version.into(),
        ResourceValue::integer(pods, 6),
        ResourceValue::integer(containers, 6),
        ready.into(),
    ];

    if statistics.has_metrics {
        let cpu_usage = get_cpu_usage(statistics, name, status["allocatable"]["cpu"].as_str());
        let mem_usage = get_mem_usage(statistics, name, status["allocatable"]["memory"].as_str());

        values.push(statistics.node(name).and_then(|n| n.metrics).map(|m| m.cpu).into());
        values.push(statistics.node(name).and_then(|n| n.metrics).map(|m| m.memory).into());
        values.push(ResourceValue::number(cpu_usage, 7));
        values.push(ResourceValue::number(mem_usage, 7));
    }

    let tags = Box::new([
        status["allocatable"]["cpu"].as_str().map(String::from).unwrap_or_default(),
        status["allocatable"]["memory"].as_str().map(String::from).unwrap_or_default(),
    ]);

    ResourceData {
        extra_values: values.into_boxed_slice(),
        is_ready: !is_terminating && is_ready,
        is_terminating,
        tags,
        ..Default::default()
    }
}

/// Returns [`Header`] for the `nodes` kubernetes resource.
pub fn header(has_metrics: bool) -> Header {
    let mut columns = vec![
        Column::fixed("TAINTS", 2, true),
        Column::bound("ROLE", 6, 30, false),
        Column::bound("VERSION", 15, 30, false),
        Column::fixed("PODS", 5, true),
        Column::fixed("CONTAINERS", 5, true),
        Column::bound("STATUS", 8, 25, false),
    ];

    let mut symbols = vec![' ', 'N', 'T', 'R', 'V', 'P', 'O', 'S'];

    if has_metrics {
        columns.push(Column::bound("CPU", 6, 20, true));
        columns.push(Column::bound("MEM", 6, 20, true));
        columns.push(Column::fixed("%CPU", 7, true));
        columns.push(Column::fixed("%MEM", 7, true));

        symbols.extend_from_slice(&['C', 'M', 'U', 'E']);
    }

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
        let name = &item.data.name;
        if let Some(data) = &mut item.data.data
            && data.tags.len() == 2
            && data.extra_values.len() == COLUMNS_NO_WITH_STATS
        {
            let cpu_usage = get_cpu_usage(statistics, name, get_as_option(&data.tags[0]));
            let mem_usage = get_mem_usage(statistics, name, get_as_option(&data.tags[1]));

            data.extra_values[6] = statistics.node(name).and_then(|n| n.metrics).map(|m| m.cpu).into();
            data.extra_values[7] = statistics.node(name).and_then(|n| n.metrics).map(|m| m.memory).into();
            data.extra_values[8] = ResourceValue::number(cpu_usage, 7);
            data.extra_values[9] = ResourceValue::number(mem_usage, 7);
        }
    }
}

fn get_first_status(conditions: Option<&Vec<Value>>) -> Option<&str> {
    conditions?
        .iter()
        .find(|c| c["status"].as_str() == Some("True"))
        .and_then(|c| c["type"].as_str())
}

fn get_roles(labels: Option<&BTreeMap<String, String>>) -> Option<String> {
    labels.map(|labels| {
        labels
            .iter()
            .filter(|(l, v)| l.starts_with("node-role.kubernetes.io/") && *v == "true")
            .map(|(l, _)| &l[24..])
            .collect::<Vec<_>>()
            .join(",")
    })
}

fn get_as_option(value: &str) -> Option<&str> {
    if !value.is_empty() { Some(value) } else { None }
}

fn get_cpu_usage(stats: &Statistics, node_name: &str, total_cpu: Option<&str>) -> Option<f64> {
    let cpu = i64::try_from(stats.node_cpu(node_name)).ok()?;
    let total = total_cpu.unwrap_or_default().parse::<CpuMetrics>().ok()?;

    Some((cpu * 100) as f64 / total.value as f64)
}

fn get_mem_usage(stats: &Statistics, node_name: &str, total_mem: Option<&str>) -> Option<f64> {
    let memory = i64::try_from(stats.node_memory(node_name)).ok()?;
    let total = total_mem?.parse::<MemoryMetrics>().ok()?;

    Some((memory * 100) as f64 / total.value as f64)
}
