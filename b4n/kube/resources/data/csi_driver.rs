use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::{kube::resources::ResourceData, ui::widgets::table::Cell};

/// Returns [`ResourceData`] for the `csidriver` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let spec = &object.data["spec"];
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [Cell; 6] = [
        spec["attachRequired"].as_bool().unwrap_or_default().into(),
        spec["podInfoOnMount"].as_bool().unwrap_or_default().into(),
        spec["storageCapacity"].as_bool().unwrap_or_default().into(),
        get_token_requests(spec["tokenRequests"].as_array()).into(),
        spec["requiresRepublish"].as_bool().unwrap_or_default().into(),
        get_modes(spec["volumeLifecycleModes"].as_array()).into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `csidriver` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("ATTACH REQUIRED", 15, false),
            Column::fixed("POD INFO", 10, false),
            Column::fixed("STORAGE CAPACITY", 16, false),
            Column::bound("TOKEN REQUESTS", 14, 25, false),
            Column::fixed("REQUIRES REPUBLISH", 18, false),
            Column::bound("MODES", 8, 25, false),
        ])),
        Rc::new([' ', 'N', 'C', 'P', 'S', 'T', 'R', 'M', 'A']),
    )
}

fn get_token_requests(token_requests: Option<&Vec<Value>>) -> Option<String> {
    let result: Vec<_> = token_requests?.iter().filter_map(|tr| tr["audience"].as_str()).collect();
    (!result.is_empty()).then_some(result.join(","))
}

fn get_modes(modes: Option<&Vec<Value>>) -> String {
    let mut modes: Vec<_> = modes.into_iter().flatten().filter_map(|mode| mode.as_str()).collect();
    if modes.is_empty() {
        return "Persistent".to_owned();
    }

    modes.sort_unstable();
    modes.join(",")
}
