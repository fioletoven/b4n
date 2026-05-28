use b4n_kube::{CONTAINERS, InitData, ObserverResult, ResourceRef};
use b4n_tui::table::ViewType;
use k8s_openapi::serde_json::Value;
use kube::ResourceExt;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::core::SharedAppData;
use crate::kube::resources::{ColumnsLayout, ResourceItem, ResourcesList};
use crate::ui::views::describe::utils::property;
use crate::ui::{presentation::ListViewer, presentation::StyledLine, views::describe::data::SectionData};

/// Returns additional describe sections for `pod` resource.
pub fn create_additional_sections(_resource: &ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let mut viewer = ListViewer::new(
        Rc::clone(app_data),
        ResourcesList::default()
            .with_columns_layout(ColumnsLayout::Compact)
            .with_focus(false),
        ViewType::Compact,
    )
    .with_no_border()
    .with_focus(false);
    viewer.table.table.limit_offset(false);

    let colors = &app_data.borrow().theme.colors.syntax.describe;

    vec![
        SectionData::Text(vec![StyledLine::default(), property(colors, "Containers", "")]),
        SectionData::List(Box::new(viewer)),
    ]
}

/// Updates additional describe sections for `pod` resource.
pub fn update_additional_sections(
    resource: &ResourceRef,
    _app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 2 {
        return;
    }

    let SectionData::List(list) = &mut sections[1] else {
        return;
    };

    let resource = ResourceRef::containers(object.name_any(), resource.namespace.clone());
    let init_data = InitData::simple(resource, "Container".to_owned(), CONTAINERS.to_owned());
    list.table.update(ObserverResult::Init(Box::new(init_data)));

    add_containers(list, object, "initContainers", "initContainerStatuses", true);
    add_containers(list, object, "containers", "containerStatuses", false);

    list.table.update(ObserverResult::InitDone);
}

fn add_containers(
    list: &mut ListViewer<ResourcesList>,
    object: &DynamicObject,
    spec_array: &str,
    status_array: &str,
    is_init_container: bool,
) {
    if let Some(containers) = object.data["spec"][spec_array].as_array() {
        for container in containers {
            let status = get_container_status(object, status_array, container);
            let resource = ResourceItem::from_container(container, status, &object.metadata, None, is_init_container);
            list.table.update(ObserverResult::new(resource, false));
        }
    }
}

fn get_container_status<'a>(object: &'a DynamicObject, status_array: &str, container: &Value) -> Option<&'a Value> {
    object.data["status"][status_array].as_array().and_then(|statuses| {
        statuses
            .iter()
            .find(|status| status["name"].as_str() == container["name"].as_str())
    })
}
