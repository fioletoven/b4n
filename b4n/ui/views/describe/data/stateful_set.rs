use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::pod::POD_SECTIONS_COUNT;
use crate::ui::views::describe::data::{SectionData, SectionDataExt, pod};
use crate::ui::views::describe::utils::{selector, update_strategy, value_to_string};

/// Returns additional describe sections for `statefulset` resource.
pub fn create_additional_sections(resource: &b4n_kube::ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let mut sections = vec![SectionData::Text(Vec::new(), 0)];
    sections.append(&mut pod::create_additional_sections(resource, app_data).with_indent(2));
    sections
}

/// Updates additional describe sections for `statefulset` resource.
pub fn update_additional_sections(
    resource: &b4n_kube::ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 1 + POD_SECTIONS_COUNT {
        return;
    }

    let SectionData::Text(lines, _) = &mut sections[0] else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let spec = &object.data["spec"];
    let mut builder = TextSectionBuilder::new(colors, lines);

    builder.start_section("State", 0, 2, Some(23));
    builder.add_str("ServiceName", spec["serviceName"].as_str());
    builder.add_str("Selector", selector(spec["selector"].as_object()));
    builder.add_str("Replicas", statefulset_replicas(object));
    builder.add_str("PodManagementPolicy", spec["podManagementPolicy"].as_str());
    builder.add_str("UpdateStrategy", update_strategy(spec["updateStrategy"].as_object()));
    builder.add_str(
        "PVCPolicy",
        pvc_retention_policy(spec["persistentVolumeClaimRetentionPolicy"].as_object()),
    );
    builder.add_num("MinReadySeconds", spec["minReadySeconds"].as_i64().map(|s| s.to_string()));
    builder.add_num(
        "Revision History Limit",
        spec["revisionHistoryLimit"].as_i64().map(|l| l.to_string()),
    );
    builder.add_str("Current Revision", object.data["status"]["currentRevision"].as_str());
    builder.add_str("Update Revision", object.data["status"]["updateRevision"].as_str());

    builder.start_section("Pod Template", 0, 0, None);
    pod::update_additional_sections(resource, app_data, object, &mut sections[1..], true);
}

fn statefulset_replicas(object: &DynamicObject) -> Option<String> {
    let desired = object.data["spec"]["replicas"].as_i64().unwrap_or(1);
    let current = object.data["status"]["replicas"].as_i64().unwrap_or_default();
    let ready = object.data["status"]["readyReplicas"].as_i64().unwrap_or_default();
    let available = object.data["status"]["availableReplicas"].as_i64().unwrap_or_default();
    let updated = object.data["status"]["updatedReplicas"].as_i64().unwrap_or_default();

    Some(format!(
        "{desired} desired | {current} current | {ready} ready | {available} available | {updated} updated"
    ))
}

fn pvc_retention_policy(policy: Option<&Map<String, Value>>) -> Option<String> {
    let policy = policy?;
    let when_deleted = policy.get("whenDeleted").and_then(value_to_string);
    let when_scaled = policy.get("whenScaled").and_then(value_to_string);

    let policy = [
        when_deleted.map(|value| format!("when deleted: {value}")),
        when_scaled.map(|value| format!("when scaled: {value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    (!policy.is_empty()).then_some(policy.join(", "))
}
