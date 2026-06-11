use b4n_kube::ResourceRef;
use k8s_openapi::serde_json::{Map, Value};
use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::SectionData;
use crate::ui::views::describe::utils::{map_join, value_to_string};

/// Returns additional describe sections for `persistentvolume` resource.
pub fn create_additional_sections(_resource: &ResourceRef, _app_data: &SharedAppData) -> Vec<SectionData> {
    vec![SectionData::Text(Vec::new(), 0)]
}

/// Updates additional describe sections for `persistentvolume` resource.
pub fn update_additional_sections(
    _resource: &ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 1 {
        return;
    }

    let SectionData::Text(lines, _) = &mut sections[0] else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let mut builder = TextSectionBuilder::new(colors, lines);

    builder.start_section("Storage", 0, 2, Some(22));
    builder.add_str("Status", object.data["status"]["phase"].as_str());
    builder.add_str("Claim", claim(object.data["spec"]["claimRef"].as_object()));
    builder.add_str("StorageClass", object.data["spec"]["storageClassName"].as_str());
    builder.add_num("Capacity", object.data["spec"]["capacity"]["storage"].as_str());
    builder.add_str(
        "AccessModes",
        map_join(object.data["spec"]["accessModes"].as_array(), value_to_string),
    );
    builder.add_str("VolumeMode", object.data["spec"]["volumeMode"].as_str());
    builder.add_str(
        "Reclaim Policy",
        object.data["spec"]["persistentVolumeReclaimPolicy"].as_str(),
    );
    builder.add_str(
        "MountOptions",
        map_join(object.data["spec"]["mountOptions"].as_array(), value_to_string),
    );
    builder.add_str(
        "VolumeAttributesClass",
        object.data["spec"]["volumeAttributesClassName"].as_str(),
    );
}

fn claim(claim_ref: Option<&Map<String, Value>>) -> Option<String> {
    let claim_ref = claim_ref?;
    let namespace = claim_ref.get("namespace").and_then(value_to_string);
    let name = claim_ref.get("name").and_then(value_to_string);
    let claim = [namespace, name].into_iter().flatten().collect::<Vec<_>>();
    (!claim.is_empty()).then_some(claim.join("/"))
}
