use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{kube::resources::ResourceData, ui::widgets::table::Cell};

/// Returns [`ResourceData`] for the `prioritylevelconfiguration` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let limited = &object.data["spec"]["limited"];
    let queuing = &limited["limitResponse"]["queuing"];
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [Cell; 5] = [
        object.data["spec"]["type"].as_str().into(),
        Cell::integer(get_nominal_shares(limited), 7),
        Cell::integer(queuing["queues"].as_i64(), 6),
        Cell::integer(queuing["handSize"].as_i64(), 6),
        Cell::integer(queuing["queueLengthLimit"].as_i64(), 8),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `prioritylevelconfiguration` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("TYPE", 8, 12, false),
            Column::fixed("NOMINAL", 8, true),
            Column::fixed("QUEUES", 7, true),
            Column::fixed("HAND SIZE", 10, true),
            Column::fixed("QUEUE LENGTH LIMIT", 19, true),
        ])),
        Rc::new([' ', 'N', 'T', 'O', 'Q', 'H', 'U', 'A']),
    )
}

fn get_nominal_shares(limited: &Value) -> Option<i64> {
    limited["nominalConcurrencyShares"]
        .as_i64()
        .or_else(|| limited["assuredConcurrencyShares"].as_i64())
}
