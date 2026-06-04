use b4n_kube::ResourceRef;
use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::kube::resources::ResourcesList;
use crate::ui::presentation::{ListViewer, StyledLine};
use crate::ui::widgets::table::BasicTable;

pub mod node;
pub mod pod;
pub mod service;

/// Holds section's data.
pub enum SectionData {
    Text(Vec<StyledLine>),
    Resources(Box<ListViewer<ResourcesList>>),
    List(Box<ListViewer<BasicTable>>),
}

/// Creates new additional sections for describe view for the specified resource.
pub fn create_additional_sections(resource: &ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    match resource.kind.name() {
        "nodes" => node::create_additional_sections(resource, app_data),
        "pods" => pod::create_additional_sections(resource, app_data),
        "services" => service::create_additional_sections(resource, app_data),
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
        "nodes" => node::update_additional_sections(resource, app_data, object, sections),
        "pods" => pod::update_additional_sections(resource, app_data, object, sections),
        "services" => service::update_additional_sections(resource, app_data, object, sections),
        _ => (),
    }
}
