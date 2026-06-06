use b4n_kube::ResourceRef;
use b4n_kube::stats::{CpuMetrics, MemoryMetrics};
use b4n_kube::utils::get_node_roles;
use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;
use std::str::FromStr;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::SectionData;
use crate::ui::views::describe::utils::value_to_string;

/// Returns additional describe sections for `node` resource.
pub fn create_additional_sections(_resource: &ResourceRef, _app_data: &SharedAppData) -> Vec<SectionData> {
    vec![SectionData::Text(Vec::new())]
}

/// Updates additional describe sections for `node` resource.
pub fn update_additional_sections(
    _resource: &ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 1 {
        return;
    }

    let SectionData::Text(lines) = &mut sections[0] else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let mut builder = TextSectionBuilder::new(colors, lines);

    add_networking_section(&mut builder, object);
    add_system_section(&mut builder, object);
    add_scheduling_section(&mut builder, object);

    let capacity = object.data["status"]["capacity"].as_object();
    add_resource_section(&mut builder, "Capacity", capacity);

    let allocatable = object.data["status"]["allocatable"].as_object();
    add_resource_section(&mut builder, "Allocatable", allocatable);
}

fn add_networking_section(builder: &mut TextSectionBuilder, object: &DynamicObject) {
    builder.start_section("Networking", 2, Some(12));
    builder.add_str("Hostname", find_node_address(object, "Hostname"));
    builder.add_str("Internal IP", find_node_address(object, "InternalIP"));
    builder.add_str("External IP", find_node_address(object, "ExternalIP"));
    builder.add_str("Pod CIDR", object.data["spec"]["podCIDR"].as_str());
    builder.add_str(
        "Pod CIDRs",
        join_filtered(object.data["spec"]["podCIDRs"].as_array(), value_to_string).as_deref(),
    );
    builder.add_str(
        "Addresses",
        node_addresses(object.data["status"]["addresses"].as_array()).as_deref(),
    );
}

fn add_system_section(builder: &mut TextSectionBuilder, object: &DynamicObject) {
    if let Some(node_info) = object.data["status"]["nodeInfo"].as_object() {
        builder.start_section("System Info", 2, Some(18));
        builder.add_str("Machine ID", node_info["machineID"].as_str());
        builder.add_str("System UUID", node_info["systemUUID"].as_str());
        builder.add_str("Boot ID", node_info["bootID"].as_str());
        builder.add_str("Kernel", node_info["kernelVersion"].as_str());
        builder.add_str("OS Image", node_info["osImage"].as_str());
        builder.add_str("OS", node_info["operatingSystem"].as_str());
        builder.add_str("Architecture", node_info["architecture"].as_str());
        builder.add_str("Container Runtime", node_info["containerRuntimeVersion"].as_str());
        builder.add_str("Kubelet", node_info["kubeletVersion"].as_str());
        builder.add_str("Kube-Proxy", node_info["kubeProxyVersion"].as_str());
    }

    builder.start_empty(0, None);
    builder.add_str("ProviderID", object.data["spec"]["providerID"].as_str());
}

fn add_scheduling_section(builder: &mut TextSectionBuilder, object: &DynamicObject) {
    builder.start_section("Scheduling", 2, Some(14));
    builder.add_bool("Unschedulable", object.data["spec"]["unschedulable"].as_bool());
    builder.add_str("Roles", get_node_roles(object, ", ").as_deref());
    builder.add_str("Taints", node_taints(object.data["spec"]["taints"].as_array()).as_deref());
}

fn add_resource_section(builder: &mut TextSectionBuilder, title: &str, source: Option<&Map<String, Value>>) {
    let Some(source) = source else {
        return;
    };

    let width = source.keys().map(String::len).max().unwrap_or_default() + 1;
    builder.start_section(title, 2, Some(width));
    for (key, value) in source {
        builder.add_num(key, format_metrics(key, value).as_deref());
    }
}

fn node_taints(values: Option<&Vec<Value>>) -> Option<String> {
    let taints = values?
        .iter()
        .map(|item| {
            let key = item["key"].as_str().unwrap_or_default();
            let value = item["value"].as_str().unwrap_or_default();
            let effect = item["effect"].as_str().unwrap_or_default();
            if value.is_empty() {
                format!("{key}:{effect}")
            } else {
                format!("{key}={value}:{effect}")
            }
        })
        .filter(|value| !value.trim_matches(':').is_empty())
        .collect::<Vec<_>>();
    (!taints.is_empty()).then_some(taints.join(", "))
}

fn format_metrics(key: &str, value: &Value) -> Option<String> {
    let value = value_to_string(value)?;

    Some(match key {
        "cpu" => CpuMetrics::from_str(&value).map(CpuMetrics::millicores).unwrap_or(value),
        "memory" | "ephemeral-storage" => MemoryMetrics::from_str(&value)
            .map(|metrics| metrics.rounded())
            .unwrap_or(value),
        _ if key.starts_with("hugepages-") => MemoryMetrics::from_str(&value)
            .map(|metrics| metrics.rounded())
            .unwrap_or(value),
        _ => value,
    })
}

fn find_node_address<'a>(object: &'a DynamicObject, address_type: &str) -> Option<&'a str> {
    object.data["status"]["addresses"].as_array().and_then(|addresses| {
        addresses
            .iter()
            .find(|address| address["type"].as_str() == Some(address_type))
            .and_then(|address| address["address"].as_str())
    })
}

fn node_addresses(values: Option<&Vec<Value>>) -> Option<String> {
    join_filtered(values, |item| {
        Some(format!("{}={}", item["type"].as_str()?, item["address"].as_str()?))
    })
}

fn join_filtered<F>(values: Option<&Vec<Value>>, filter: F) -> Option<String>
where
    F: Fn(&Value) -> Option<String>,
{
    let filtered = values?.iter().filter_map(filter).collect::<Vec<_>>();
    (!filtered.is_empty()).then_some(filtered.join(", "))
}
