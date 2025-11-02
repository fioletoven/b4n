use b4n_kube::stats::Metrics;
use b4n_tui::grid::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use std::rc::Rc;

use crate::kubernetes::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the pod's `container`.
pub fn data(
    container: &Value,
    status: Option<&Value>,
    metrics: Option<Metrics>,
    is_init_container: bool,
    is_terminating: bool,
) -> ResourceData {
    let restarts = status.and_then(|s| s.get("restartCount")).and_then(Value::as_i64);
    let ready = status
        .and_then(|s| s.get("ready"))
        .and_then(Value::as_bool)
        .unwrap_or_default();

    let completed = status.and_then(|s| s["state"]["terminated"]["reason"].as_str());
    let is_running = status.and_then(|s| s.get("state")).and_then(|s| s.get("running")).is_some();
    let phase = if is_running {
        "Running"
    } else if let Some(completed) = completed {
        completed
    } else {
        status
            .and_then(|s| s["state"]["waiting"]["reason"].as_str())
            .unwrap_or("Unknown")
    };

    let mut values = vec![
        ResourceValue::integer(restarts, 5),
        ready.into(),
        phase.into(),
        is_init_container.into(),
        container["image"].as_str().into(),
    ];

    if let Some(metrics) = metrics {
        values.push(metrics.cpu.into());
        values.push(metrics.memory.into());
    }

    ResourceData {
        extra_values: values.into_boxed_slice(),
        is_completed: completed.is_some(),
        is_ready: is_running,
        is_terminating,
        ..Default::default()
    }
}

/// Returns [`Header`] for the pod's `container`.
pub fn header(has_metrics: bool) -> Header {
    let mut columns = vec![
        Column::fixed("RESTARTS", 3, true),
        Column::fixed("READY", 7, false),
        Column::bound("STATE", 10, 20, false),
        Column::fixed("INIT", 6, false),
        Column::bound("IMAGE", 15, 70, false),
    ];

    let mut symbols = vec![' ', 'N', 'R', 'E', 'S', 'T', 'I'];

    if has_metrics {
        columns.push(Column::bound("CPU", 5, 15, true));
        columns.push(Column::bound("MEM", 5, 15, true));
        symbols.push('C');
        symbols.push('M');
    }

    symbols.push('A');

    Header::from(
        NAMESPACE,
        Some(columns.into_boxed_slice()),
        Rc::from(symbols.into_boxed_slice()),
    )
}
