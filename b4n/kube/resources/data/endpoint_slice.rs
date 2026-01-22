use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::collections::HashSet;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `endpointslice` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let values: [ResourceValue; 3] = [
        object.data["addressType"].as_str().into(),
        get_ports(object.data["ports"].as_array()).into(),
        get_endpoints(object.data["endpoints"].as_array()).into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `endpointslice` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::fixed("ADDRESS TYPE", 15, false),
            Column::bound("PORTS", 15, 50, false),
            Column::bound("ENDPOINTS", 15, 50, false),
        ])),
        Rc::new([' ', 'N', 'A', 'P', 'E', 'A']),
    )
}

fn get_ports(ports: Option<&Vec<Value>>) -> Option<String> {
    let ports = ports?
        .iter()
        .filter_map(|port| port["port"].as_u64())
        .map(|port| port.to_string())
        .collect::<HashSet<String>>();

    if ports.is_empty() {
        return None;
    }

    let mut sorted = ports.into_iter().collect::<Vec<_>>();
    sorted.sort();

    Some(sorted.join(","))
}

fn get_endpoints(endpoints: Option<&Vec<Value>>) -> Option<String> {
    let addresses = endpoints?
        .iter()
        .flat_map(|endpoint| {
            endpoint["addresses"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|addr| addr.as_str())
        })
        .collect::<HashSet<&str>>();

    if addresses.is_empty() {
        return None;
    }

    let mut sorted = addresses.into_iter().collect::<Vec<_>>();
    sorted.sort();

    Some(sorted.join(","))
}
