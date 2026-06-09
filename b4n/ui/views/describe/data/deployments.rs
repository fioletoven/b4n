use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::{SectionData, SectionDataExt, pod};
use crate::ui::views::describe::utils::{selector, value_to_string};

/// Returns additional describe sections for `deployment` resource.
pub fn create_additional_sections(resource: &b4n_kube::ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let mut sections = vec![SectionData::Text(Vec::new(), 0)];

    let mut pod_sections = pod::create_additional_sections(resource, app_data);
    pod_sections.set_indent(2);

    sections.append(&mut pod_sections);
    sections
}

/// Updates additional describe sections for `deployment` resource.
pub fn update_additional_sections(
    resource: &b4n_kube::ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 7 {
        return;
    }

    let SectionData::Text(lines, _) = &mut sections[0] else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let spec = &object.data["spec"];
    let mut builder = TextSectionBuilder::new(colors, lines);

    builder.start_section("Rollout", 0, 2, Some(25));
    builder.add_str("Selector", selector(spec["selector"].as_object()).as_deref());
    builder.add_str("Replicas", deployment_replicas(object).as_deref());
    builder.add_str("StrategyType", spec["strategy"]["type"].as_str());
    builder.add_str(
        "RollingUpdate",
        rolling_update_strategy(spec["strategy"]["rollingUpdate"].as_object()).as_deref(),
    );
    builder.add_num("MinReadySeconds", spec["minReadySeconds"].as_i64().map(|s| s.to_string()));
    builder.add_num(
        "ProgressDeadlineSeconds",
        spec["progressDeadlineSeconds"].as_i64().map(|s| s.to_string()),
    );
    builder.add_num(
        "RevisionHistoryLimit",
        spec["revisionHistoryLimit"].as_i64().map(|l| l.to_string()),
    );
    builder.add_bool("Paused", spec["paused"].as_bool());

    builder.start_section("Pod Template", 0, 0, None);
    pod::update_additional_sections(resource, app_data, object, &mut sections[1..], true);
}

fn deployment_replicas(object: &DynamicObject) -> Option<String> {
    let desired = object.data["spec"]["replicas"].as_i64().unwrap_or(1);
    let updated = object.data["status"]["updatedReplicas"].as_i64().unwrap_or_default();
    let total = object.data["status"]["replicas"].as_i64().unwrap_or_default();
    let available = object.data["status"]["availableReplicas"].as_i64().unwrap_or_default();
    let unavailable = object.data["status"]["unavailableReplicas"].as_i64().unwrap_or_default();

    Some(format!(
        "{desired} desired | {updated} updated | {total} total | {available} available | {unavailable} unavailable"
    ))
}

fn rolling_update_strategy(strategy: Option<&Map<String, Value>>) -> Option<String> {
    let strategy = strategy?;
    let max_unavailable = strategy.get("maxUnavailable").and_then(value_to_string);
    let max_surge = strategy.get("maxSurge").and_then(value_to_string);

    let strategy = [
        max_unavailable.map(|value| format!("{value} max unavailable")),
        max_surge.map(|value| format!("{value} max surge")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    (!strategy.is_empty()).then_some(strategy.join(", "))
}
