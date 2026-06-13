use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::pod::POD_SECTIONS_COUNT;
use crate::ui::views::describe::data::{SectionData, SectionDataExt, pod};
use crate::ui::views::describe::utils::{selector, update_strategy};

/// Returns additional describe sections for `daemonset` resource.
pub fn create_additional_sections(resource: &b4n_kube::ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let mut sections = vec![SectionData::Text(Vec::new(), 0)];
    sections.append(&mut pod::create_additional_sections(resource, app_data).with_indent(2));
    sections
}

/// Updates additional describe sections for `daemonset` resource.
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

    builder.start_section("Scheduling", 0, 2, Some(16));
    builder.add_str("Selector", selector(spec["selector"].as_object()));
    builder.add_str("Pods", Some(daemonset_pods(object)));
    builder.add_str("UpdateStrategy", update_strategy(spec["updateStrategy"].as_object()));
    builder.add_inum("MinReadySeconds", spec["minReadySeconds"].as_i64());

    builder.start_section("Pod Template", 0, 0, None);
    pod::update_additional_sections(resource, app_data, object, &mut sections[1..], true);
}

fn daemonset_pods(object: &DynamicObject) -> String {
    let desired = object.data["status"]["desiredNumberScheduled"].as_i64().unwrap_or_default();
    let current = object.data["status"]["currentNumberScheduled"].as_i64().unwrap_or_default();
    let ready = object.data["status"]["numberReady"].as_i64().unwrap_or_default();
    let available = object.data["status"]["numberAvailable"].as_i64().unwrap_or_default();
    let up_to_date = object.data["status"]["updatedNumberScheduled"].as_i64().unwrap_or_default();
    let misscheduled = object.data["status"]["numberMisscheduled"].as_i64().unwrap_or_default();

    format!(
        "{desired} desired | {current} current | {ready} ready | {available} available | {up_to_date} up-to-date | {misscheduled} misscheduled"
    )
}
