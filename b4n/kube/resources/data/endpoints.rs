use b4n_tui::table::{Column, Header, NAMESPACE};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::kube::resources::{ResourceData, ResourceValue};

/// Returns [`ResourceData`] for the `endpoints` kubernetes resource.
pub fn data(object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();
    let values: [ResourceValue; 1] = [get_endpoints(object.data["subsets"].as_array()).into()];

    ResourceData::new(Box::new(values), is_terminating)
}

/// Returns [`Header`] for the `endpoints` kubernetes resource.
pub fn header() -> Header {
    Header::from(
        NAMESPACE,
        Some(Box::new([Column::bound("ENDPOINTS", 15, 50, false)])),
        Rc::new([' ', 'N', 'E', 'A']),
    )
}

fn get_endpoints(subsets: Option<&Vec<Value>>) -> Option<String> {
    let mut endpoints = HashMap::<&str, HashSet<u64>>::new();

    for subset in subsets? {
        let Some(addresses) = subset["addresses"]
            .as_array()
            .map(|a| a.iter().filter_map(|a| a["ip"].as_str()).collect::<HashSet<_>>())
        else {
            continue;
        };

        let Some(ports) = subset["ports"]
            .as_array()
            .map(|a| a.iter().filter_map(|a| a["port"].as_u64()).collect::<HashSet<_>>())
        else {
            continue;
        };

        for address in addresses {
            endpoints.entry(address).or_default().extend(ports.iter());
        }
    }

    if endpoints.is_empty() {
        None
    } else {
        let mut result = String::new();

        let mut endpoints = endpoints.into_iter().collect::<Vec<_>>();
        endpoints.sort_by_key(|(ip, _)| *ip);

        for (i, (address, ports)) in endpoints.iter().enumerate() {
            let mut ports = ports.iter().map(|p| p.to_string()).collect::<Vec<_>>();
            ports.sort();

            if i > 0 {
                result.push(' ');
            }

            result.push_str(address);
            result.push(':');
            result.push_str(&ports.join(","));
        }

        Some(result)
    }
}
