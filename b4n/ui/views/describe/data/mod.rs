use b4n_kube::ResourceRef;
use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::kube::resources::ResourcesList;
use crate::ui::presentation::{ListViewer, StyledLine};

pub mod node;
pub mod pod;

/// Holds section's data.
pub enum SectionData {
    Text(Vec<StyledLine>),
    List(Box<ListViewer<ResourcesList>>),
}

/// Creates new additional sections for describe view for the specified resource.
pub fn create_additional_sections(resource: &ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    match resource.kind.name() {
        "pods" => pod::create_additional_sections(resource, app_data),
        "nodes" => node::create_additional_sections(resource, app_data),
        _ => Vec::new(),
    }
}

/// Updates additional sections for describe view for the specified resource.
pub fn update_additional_sections(
    resource: &ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    match resource.kind.name() {
        "pods" => pod::update_additional_sections(resource, app_data, object, sections),
        "nodes" => node::update_additional_sections(resource, app_data, object, sections),
        _ => (),
    }
}
