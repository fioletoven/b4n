use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `ingress` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let addresses = get_addresses(object.data["status"]["loadBalancer"]["ingress"].as_array());
    let hosts = get_hosts(object.data["spec"]["rules"].as_array());
    let ports = get_ports(object.data["spec"]["tls"].is_array());
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    let values: [ResourceValue; 4] = [
        object.data["spec"]["ingressClassName"].as_str().into(),
        hosts.into(),
        addresses.into(),
        ports.into(),
    ];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `ingress` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("CLASS", 10, 25, false),
            Column::bound("HOSTS", 10, 30, false),
            Column::bound("ADDRESSES", 8, 30, false),
            Column::fixed("PORTS", 8, false),
        ])),
        Rc::new([' ', 'N', 'C', 'H', 'D', 'A']),
    )
}

fn get_addresses(ingresses: Option<&Vec<Value>>) -> Option<String> {
    Some(
        ingresses?
            .iter()
            .filter_map(|i| i["ip"].as_str())
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn get_hosts(rules: Option<&Vec<Value>>) -> Option<String> {
    Some(rules?.iter().filter_map(|i| i["host"].as_str()).collect::<Vec<_>>().join(","))
}

fn get_ports(has_tls: bool) -> String {
    if has_tls { "80,443".to_string() } else { "80".to_string() }
}
