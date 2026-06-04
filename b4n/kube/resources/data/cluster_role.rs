use b4n_tui::table::{Column, Header, NAMESPACE};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{kube::resources::ResourceData, ui::widgets::table::Cell};

/// Returns [`ResourceData`] for the `clusterrole` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let rules = object.data["rules"].as_array().map_or(0, Vec::len);
    let aggregates = object.data["aggregationRule"]["clusterRoleSelectors"]
        .as_array()
        .map_or(0, Vec::len);
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [Cell; 2] = [rules.to_string().into(), aggregates.to_string().into()];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `clusterrole` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("RULES", 6, true),
            Column::fixed("AGGREGATES", 11, true),
        ])),
        Rc::new([' ', 'N', 'R', 'G', 'A']),
    )
}
