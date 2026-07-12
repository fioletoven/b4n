use b4n_kube::stats::Metrics;
use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use std::{fmt::Display, rc::Rc};

use crate::{kube::resources::ResourceData, ui::widgets::table::Cell};

/// Pod container type.
#[derive(Clone, Copy, PartialEq)]
pub enum ContainerType {
    Regular,
    Init,
    Ephemeral,
}

impl Display for ContainerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerType::Regular => write!(f, "Regular"),
            ContainerType::Init => write!(f, "Init"),
            ContainerType::Ephemeral => write!(f, "Ephemeral"),
        }
    }
}

/// Returns [`ResourceData`] for the pod's `container`.
pub fn data(
    container: &Value,
    status: Option<&Value>,
    metrics: Option<Metrics>,
    container_type: ContainerType,
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
        Cell::integer(restarts, 5),
        ready.into(),
        phase.into(),
        container_type.to_string().into(),
    ];

    if let Some(metrics) = metrics {
        values.push(metrics.cpu.into());
        values.push(metrics.memory.into());
    }

    values.push(container["image"].as_str().into());

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
        Column::bound("TYPE", 7, 9, false),
    ];

    let mut symbols = vec![' ', 'N', 'R', 'E', 'S', 'T'];

    if has_metrics {
        columns.push(Column::bound("CPU", 5, 15, true));
        columns.push(Column::bound("MEM", 5, 15, true));
        symbols.push('C');
        symbols.push('M');
    }

    columns.push(Column::bound("IMAGE", 15, 70, false));
    symbols.push('I');
    symbols.push('A');

    Header::from(
        NAMESPACE,
        Some(columns.into_boxed_slice()),
        Rc::from(symbols.into_boxed_slice()),
    )
    .with_stretch_last()
}
