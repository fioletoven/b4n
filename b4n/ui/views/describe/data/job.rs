use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::pod::POD_SECTIONS_COUNT;
use crate::ui::views::describe::data::{SectionData, SectionDataExt, pod};
use crate::ui::views::describe::utils::selector;

/// Returns additional describe sections for `job` resource.
pub fn create_additional_sections(resource: &b4n_kube::ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let mut sections = vec![SectionData::Text(Vec::new(), 0)];
    sections.append(&mut pod::create_additional_sections(resource, app_data).with_indent(2));
    sections
}

/// Updates additional describe sections for `job` resource.
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

    builder.start_section("Execution", 0, 2, Some(24));
    builder.add_str("Selector", selector(spec["selector"].as_object()));
    builder.add_inum("Parallelism", spec["parallelism"].as_i64());
    builder.add_inum("Completions", spec["completions"].as_i64());
    builder.add_str("CompletionMode", spec["completionMode"].as_str());
    builder.add_str("Status", job_status(object));
    builder.add_inum("BackoffLimit", spec["backoffLimit"].as_i64());
    builder.add_inum("BackoffLimitPerIndex", spec["backoffLimitPerIndex"].as_i64());
    builder.add_inum("MaxFailedIndexes", spec["maxFailedIndexes"].as_i64());
    builder.add_inum("Active Deadline Seconds", spec["activeDeadlineSeconds"].as_i64());
    builder.add_bool("Suspend", spec["suspend"].as_bool());
    builder.add_inum("TTL After Finished", spec["ttlSecondsAfterFinished"].as_i64());
    builder.add_str("PodReplacementPolicy", spec["podReplacementPolicy"].as_str());

    builder.start_section("Pod Template", 0, 0, None);
    pod::update_additional_sections(resource, app_data, object, &mut sections[1..], true);
}

fn job_status(object: &DynamicObject) -> Option<String> {
    let active = object.data["status"]["active"].as_i64().unwrap_or_default();
    let succeeded = object.data["status"]["succeeded"].as_i64().unwrap_or_default();
    let failed = object.data["status"]["failed"].as_i64().unwrap_or_default();
    let ready = object.data["status"]["ready"].as_i64().unwrap_or_default();
    let terminating = object.data["status"]["terminating"].as_i64().unwrap_or_default();

    Some(format!(
        "{active} active | {ready} ready | {succeeded} succeeded | {failed} failed | {terminating} terminating"
    ))
}
