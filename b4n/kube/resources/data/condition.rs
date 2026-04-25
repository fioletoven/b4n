use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the kubernetes resource status `condition`.
pub fn data(condition: &Value) -> ResourceData {
    let values: [ResourceValue; 3] = [
        condition["status"].as_str().into(),
        condition["reason"].as_str().into(),
        condition["message"].as_str().into(),
    ];

    ResourceData::new(Box::new(values), false)
}

/// Returns [`Header`] for the kubernetes resource status `condition`.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("STATUS", 10, 20, false),
            Column::bound("REASON", 10, 20, false),
            Column::bound("MESSAGE", 10, 50, false),
        ])),
        Rc::new([' ', 'T', 'S', 'R', 'M', 'A']),
    )
    .with_name_column(Column::bound("TYPE", 6, 6, false))
}
