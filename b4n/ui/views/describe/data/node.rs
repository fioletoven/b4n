use b4n_config::themes::YamlSyntaxColors;
use b4n_kube::ResourceRef;
use b4n_kube::stats::{CpuMetrics, MemoryMetrics};
use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;
use std::str::FromStr;

use crate::core::SharedAppData;
use crate::ui::presentation::StyledLine;
use crate::ui::views::describe::data::SectionData;
use crate::ui::views::describe::utils::{aligned_property, property, value_to_string};

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
}

fn add_resource_section(
    lines: &mut Vec<StyledLine>,
    colors: &YamlSyntaxColors,
    title: &str,
    source: Option<&Map<String, Value>>,
) {
    lines.push(property(colors, title, ""));

    let Some(source) = source else {
        return;
    };

    let width = source.keys().map(String::len).max().unwrap_or_default() + 1;

    for (key, value) in source {
        lines.push(aligned_property(colors, key, &format_value(key, value), 2, width));
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
