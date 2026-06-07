use b4n_kube::ResourceRef;
use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::SectionData;
use crate::ui::views::describe::utils::{map_join, selector, value_to_string};

/// Returns additional describe sections for `persistentvolumeclaim` resource.
pub fn create_additional_sections(_resource: &ResourceRef, _app_data: &SharedAppData) -> Vec<SectionData> {
    vec![SectionData::Text(Vec::new())]
}

/// Updates additional describe sections for `persistentvolumeclaim` resource.
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

    builder.start_section("Storage", 0, 2, Some(22));
    builder.add_str("Status", object.data["status"]["phase"].as_str());
    builder.add_str("Volume", object.data["spec"]["volumeName"].as_str());
    builder.add_str("StorageClass", object.data["spec"]["storageClassName"].as_str());
    builder.add_str(
        "AccessModes",
        map_join(object.data["spec"]["accessModes"].as_array(), value_to_string),
    );
    builder.add_str("VolumeMode", object.data["spec"]["volumeMode"].as_str());
    builder.add_num(
        "RequestedStorage",
        object.data["spec"]["resources"]["requests"]["storage"].as_str(),
    );
    builder.add_num("Capacity", object.data["status"]["capacity"]["storage"].as_str());
    builder.add_str("DataSource", data_source(object.data["spec"]["dataSource"].as_object()));
    builder.add_str("DataSourceRef", data_source(object.data["spec"]["dataSourceRef"].as_object()));
    builder.add_str("Selector", selector(object.data["spec"]["selector"].as_object()));
}

fn data_source(source: Option<&Map<String, Value>>) -> Option<String> {
    let source = source?;
    let kind = source.get("kind").and_then(value_to_string);
    let name = source.get("name").and_then(value_to_string);
    let namespace = source.get("namespace").and_then(value_to_string);

    let source = [kind, name, namespace].into_iter().flatten().collect::<Vec<_>>();
    (!source.is_empty()).then_some(source.join(" / "))
}
