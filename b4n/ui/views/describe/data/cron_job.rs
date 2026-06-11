use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::job::JOB_SECTIONS_COUNT;
use crate::ui::views::describe::data::{SectionData, SectionDataExt, job};

/// Returns additional describe sections for `cronjob` resource.
pub fn create_additional_sections(resource: &b4n_kube::ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let mut sections = vec![SectionData::Text(Vec::new(), 0)];
    sections.append(&mut job::create_additional_sections(resource, app_data).with_indent(2));
    sections
}

/// Updates additional describe sections for `cronjob` resource.
pub fn update_additional_sections(
    resource: &b4n_kube::ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 1 + JOB_SECTIONS_COUNT {
        return;
    }

    let SectionData::Text(lines, _) = &mut sections[0] else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let spec = &object.data["spec"];
    let mut builder = TextSectionBuilder::new(colors, lines);

    builder.start_section("Schedule", 0, 2, Some(27));
    builder.add_str("Schedule", spec["schedule"].as_str());
    builder.add_str("TimeZone", spec["timeZone"].as_str());
    builder.add_str("ConcurrencyPolicy", spec["concurrencyPolicy"].as_str());
    builder.add_bool("Suspend", spec["suspend"].as_bool());
    builder.add_inum("StartingDeadlineSeconds", spec["startingDeadlineSeconds"].as_i64());
    builder.add_inum("SuccessfulJobsHistoryLimit", spec["successfulJobsHistoryLimit"].as_i64());
    builder.add_inum("FailedJobsHistoryLimit", spec["failedJobsHistoryLimit"].as_i64());
    builder.add_str("Last Schedule Time", object.data["status"]["lastScheduleTime"].as_str());
    builder.add_str("Last Successful Time", object.data["status"]["lastSuccessfulTime"].as_str());
    builder.add_inum(
        "Active Jobs",
        object.data["status"]["active"].as_array().map(|items| items.len() as i64),
    );

    builder.start_section("Job Template", 0, 0, None);
    job::update_additional_sections(resource, app_data, object, &mut sections[1..], true);
}
