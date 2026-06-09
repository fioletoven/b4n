use b4n_kube::ResourceRef;
use kube::api::DynamicObject;

use crate::core::SharedAppData;
use crate::kube::resources::ResourcesList;
use crate::ui::presentation::{ListViewer, StyledLine};
use crate::ui::widgets::table::BasicTable;

mod daemon_set;
mod deployments;
mod node;
mod persistent_volume;
mod persistent_volume_claim;
mod pod;
mod replica_set;
mod service;

/// Creates new additional sections for describe view for the specified resource.
pub fn create_additional_sections(resource: &ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    match resource.kind.name() {
        "daemonsets" => daemon_set::create_additional_sections(resource, app_data),
        "deployments" => deployments::create_additional_sections(resource, app_data),
        "nodes" => node::create_additional_sections(resource, app_data),
        "persistentvolumes" => persistent_volume::create_additional_sections(resource, app_data),
        "persistentvolumeclaims" => persistent_volume_claim::create_additional_sections(resource, app_data),
        "pods" => pod::create_additional_sections(resource, app_data),
        "replicasets" => replica_set::create_additional_sections(resource, app_data),
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
        "daemonsets" => daemon_set::update_additional_sections(resource, app_data, object, sections),
        "deployments" => deployments::update_additional_sections(resource, app_data, object, sections),
        "nodes" => node::update_additional_sections(resource, app_data, object, sections),
        "persistentvolumes" => persistent_volume::update_additional_sections(resource, app_data, object, sections),
        "persistentvolumeclaims" => persistent_volume_claim::update_additional_sections(resource, app_data, object, sections),
        "pods" => pod::update_additional_sections(resource, app_data, object, sections, false),
        "replicasets" => replica_set::update_additional_sections(resource, app_data, object, sections),
        "services" => service::update_additional_sections(resource, app_data, object, sections),
        _ => (),
    }
}

/// Holds section's data.
pub enum SectionData {
    Text(Vec<StyledLine>, u16),
    Resources(Box<ListViewer<ResourcesList>>, u16),
    List(Box<ListViewer<BasicTable>>, u16),
}

/// Extension methods for [`SectionData`].
pub trait SectionDataExt {
    /// Sets `indent` for all sections in the collection.
    fn with_indent(self, indent: u16) -> Self;
}

impl SectionDataExt for Vec<SectionData> {
    fn with_indent(mut self, indent: u16) -> Self {
        for section in &mut self {
            match section {
                SectionData::Text(_, i) => *i = indent,
                SectionData::Resources(_, i) => *i = indent,
                SectionData::List(_, i) => *i = indent,
            }
        }
        self
    }
}
