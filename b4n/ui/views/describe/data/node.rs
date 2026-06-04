use b4n_config::themes::YamlSyntaxColors;
use b4n_kube::ResourceRef;
use b4n_kube::stats::{CpuMetrics, MemoryMetrics};
use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;
use std::str::FromStr;

use crate::core::SharedAppData;
use crate::ui::presentation::StyledLine;
use crate::ui::views::describe::data::SectionData;
use crate::ui::views::describe::utils::{ValueKind, aligned_property, header, property, value_to_string};

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

    let capacity = object.data["status"]["capacity"].as_object();
    lines.push(StyledLine::default());
    add_resource_section(lines, colors, "Capacity", capacity);

    let allocatable = object.data["status"]["allocatable"].as_object();
    lines.push(StyledLine::default());
    add_resource_section(lines, colors, "Allocatable", allocatable);

    add_system_section(lines, colors, object);
}

fn add_resource_section(
    lines: &mut Vec<StyledLine>,
    colors: &YamlSyntaxColors,
    title: &str,
    source: Option<&Map<String, Value>>,
) {
    lines.push(header(colors, title));

    let Some(source) = source else {
        return;
    };

    let width = source.keys().map(String::len).max().unwrap_or_default() + 1;

    for (key, value) in source {
        let line = aligned_property(colors, key, format_value(key, value), ValueKind::Numeric, 2, width);
        lines.push(line);
    }
}

fn format_value(key: &str, value: &Value) -> String {
    let value = value_to_string(value);

    match key {
        "cpu" => CpuMetrics::from_str(&value).map(CpuMetrics::millicores).unwrap_or(value),
        "memory" | "ephemeral-storage" => MemoryMetrics::from_str(&value)
            .map(|metrics| metrics.rounded())
            .unwrap_or(value),
        _ if key.starts_with("hugepages-") => MemoryMetrics::from_str(&value)
            .map(|metrics| metrics.rounded())
            .unwrap_or(value),
        _ => value,
    }
}

fn add_system_section(lines: &mut Vec<StyledLine>, colors: &YamlSyntaxColors, object: &DynamicObject) {
    lines.push(StyledLine::default());
    let provider_id = object.data["spec"]["providerID"].as_str().unwrap_or_default();
    lines.push(property(colors, "ProviderID", provider_id, ValueKind::String, 0));

    if let Some(node_info) = object.data["status"]["nodeInfo"].as_object() {
        lines.push(StyledLine::default());
        lines.push(header(colors, "System Info"));

        lines.push(property_str(colors, "Machine ID", node_info["machineID"].as_str()));
        lines.push(property_str(colors, "System UUID", node_info["systemUUID"].as_str()));
        lines.push(property_str(colors, "Boot ID", node_info["bootID"].as_str()));
        lines.push(property_str(colors, "Kernel", node_info["kernelVersion"].as_str()));
        lines.push(property_str(colors, "OS Image", node_info["osImage"].as_str()));
        lines.push(property_str(colors, "OS", node_info["operatingSystem"].as_str()));
        lines.push(property_str(colors, "Architecture", node_info["architecture"].as_str()));
        lines.push(property_str(
            colors,
            "Container Runtime",
            node_info["containerRuntimeVersion"].as_str(),
        ));
        lines.push(property_str(colors, "Kubelet", node_info["kubeletVersion"].as_str()));
        lines.push(property_str(colors, "Kube-Proxy", node_info["kubeProxyVersion"].as_str()));
    }
}

fn property_str(colors: &YamlSyntaxColors, name: &str, value: Option<&str>) -> StyledLine {
    property(colors, name, value.unwrap_or_default(), ValueKind::String, 2)
}
