use b4n_config::themes::YamlSyntaxColors;
use b4n_kube::{CONTAINERS, InitData, ObserverResult, ResourceRef};
use b4n_tui::table::ViewType;
use k8s_openapi::serde_json::{Map, Value};
use kube::ResourceExt;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::core::SharedAppData;
use crate::kube::resources::{ColumnsLayout, ResourceItem, ResourcesList};
use crate::ui::views::describe::utils::{ValueKind, aligned_property, header, none, uppercase_first_letter, value_to_string};
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
        SectionData::Text(vec![StyledLine::default(), header(colors, "Containers")]),
        SectionData::List(Box::new(viewer)),
        SectionData::Text(vec![StyledLine::default(), header(colors, "Volumes")]),
    ]
}

/// Updates additional describe sections for `pod` resource.
pub fn update_additional_sections(
    resource: &ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 3 {
        return;
    }

    update_containers_section(resource, object, sections);
    update_volume_section(app_data, object, sections);
}

fn update_containers_section(resource: &ResourceRef, object: &DynamicObject, sections: &mut [SectionData]) {
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

fn update_volume_section(app_data: &SharedAppData, object: &DynamicObject, sections: &mut [SectionData]) {
    let SectionData::Text(lines) = &mut sections[2] else {
        return;
    };

    lines.truncate(2);

    let colors = &app_data.borrow().theme.colors.syntax.describe;

    let Some(volumes) = object.data["spec"]["volumes"].as_array() else {
        lines.push(none(colors));
        return;
    };

    if volumes.is_empty() {
        lines.push(none(colors));
        return;
    }

    for volume in volumes {
        add_volume(lines, colors, volume);
    }
}

fn add_volume(lines: &mut Vec<StyledLine>, colors: &YamlSyntaxColors, volume: &Value) {
    let Some(name) = volume["name"].as_str() else {
        return;
    };

    let properties = get_volume_properties(volume);
    let width = properties.iter().map(|(key, _, _)| key.len()).max().unwrap_or_default() + 1;

    lines.push(header(colors, format!("  {name}")));
    for (key, value, kind) in properties {
        lines.push(aligned_property(colors, key, &value, kind, 4, width));
    }
}

type TypedProperty = (&'static str, String, ValueKind);
type FieldHandlerTuple<'a> = (&'a str, fn(&Map<String, Value>) -> Vec<TypedProperty>);

fn get_volume_properties(volume: &Value) -> Vec<TypedProperty> {
    let handlers: [FieldHandlerTuple; _] = [
        ("persistentVolumeClaim", persistent_volume_claim_properties),
        ("secret", secret_properties),
        ("configMap", config_map_properties),
        ("downwardAPI", downward_api_properties),
        ("emptyDir", empty_dir_properties),
        ("hostPath", host_path_properties),
        ("nfs", nfs_properties),
        ("csi", csi_properties),
        ("image", image_properties),
        ("ephemeral", ephemeral_properties),
        ("projected", projected_properties),
    ];

    handlers
        .into_iter()
        .find_map(|(field, handler)| volume[field].as_object().map(handler))
        .or_else(|| {
            volume.as_object().and_then(|properties| {
                properties
                    .iter()
                    .find(|(key, _)| key.as_str() != "name")
                    .map(|(volume_type, _)| vec![("Type", uppercase_first_letter(volume_type), ValueKind::String)])
            })
        })
        .unwrap_or_default()
}

fn persistent_volume_claim_properties(pvc: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "PersistentVolumeClaim".to_owned(), ValueKind::String),
        ("ClaimName", string_value(pvc, "claimName"), ValueKind::String),
        ("ReadOnly", bool_value(pvc, "readOnly"), ValueKind::Boolean),
    ]
}

fn secret_properties(secret: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "Secret".to_owned(), ValueKind::String),
        ("SecretName", string_value(secret, "secretName"), ValueKind::String),
        ("Optional", bool_value(secret, "optional"), ValueKind::Boolean),
    ]
}

fn config_map_properties(config_map: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "ConfigMap".to_owned(), ValueKind::String),
        ("Name", string_value(config_map, "name"), ValueKind::String),
        ("Optional", bool_value(config_map, "optional"), ValueKind::Boolean),
    ]
}

fn downward_api_properties(downward_api: &Map<String, Value>) -> Vec<TypedProperty> {
    let items = downward_api
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.len().to_string())
        .unwrap_or_default();

    vec![
        ("Type", "DownwardAPI".to_owned(), ValueKind::String),
        ("Items", items, ValueKind::Numeric),
    ]
}

fn empty_dir_properties(empty_dir: &Map<String, Value>) -> Vec<TypedProperty> {
    let limit = empty_dir.get("sizeLimit").map(value_to_string);
    let (limit, kind) = limit.map_or_else(|| ("--unset--".to_owned(), ValueKind::Normal), |l| (l, ValueKind::String));

    vec![
        ("Type", "EmptyDir".to_owned(), ValueKind::String),
        ("Medium", string_value(empty_dir, "medium"), ValueKind::String),
        ("SizeLimit", limit, kind),
    ]
}

fn host_path_properties(host_path: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "HostPath".to_owned(), ValueKind::String),
        ("Path", string_value(host_path, "path"), ValueKind::String),
        ("HostPathType", string_value(host_path, "type"), ValueKind::String),
    ]
}

fn nfs_properties(nfs: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "NFS".to_owned(), ValueKind::String),
        ("Server", string_value(nfs, "server"), ValueKind::String),
        ("Path", string_value(nfs, "path"), ValueKind::String),
        ("ReadOnly", bool_value(nfs, "readOnly"), ValueKind::Boolean),
    ]
}

fn csi_properties(csi: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "CSI".to_owned(), ValueKind::String),
        ("Driver", string_value(csi, "driver"), ValueKind::String),
        ("FSType", string_value(csi, "fsType"), ValueKind::String),
        ("ReadOnly", bool_value(csi, "readOnly"), ValueKind::Boolean),
    ]
}

fn image_properties(image: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "Image".to_owned(), ValueKind::String),
        ("Reference", string_value(image, "reference"), ValueKind::String),
        ("PullPolicy", string_value(image, "pullPolicy"), ValueKind::String),
    ]
}

fn ephemeral_properties(ephemeral: &Map<String, Value>) -> Vec<TypedProperty> {
    let ephemeral = ephemeral
        .get("volumeClaimTemplate")
        .and_then(|template| template.get("metadata"))
        .and_then(|metadata| metadata.get("name"))
        .map(value_to_string);
    let (ephemeral, kind) = ephemeral.map_or_else(|| ("--generated--".to_owned(), ValueKind::Normal), |e| (e, ValueKind::String));

    vec![
        ("Type", "Ephemeral".to_owned(), ValueKind::String),
        ("VolumeClaimTemplate", ephemeral, kind),
    ]
}

fn projected_properties(projected: &Map<String, Value>) -> Vec<TypedProperty> {
    let mut properties = vec![("Type", "Projected".to_owned(), ValueKind::String)];

    if let Some(sources) = projected.get("sources").and_then(Value::as_array) {
        for source in sources {
            if let Some(secret) = source["secret"].as_object() {
                if let Some(name) = secret.get("name") {
                    properties.push(("SecretName", value_to_string(name), ValueKind::String));
                }

                properties.push(("Optional", bool_value(secret, "optional"), ValueKind::Boolean));
            }

            if let Some(config_map) = source["configMap"].as_object() {
                if let Some(name) = config_map.get("name") {
                    properties.push(("ConfigMapName", value_to_string(name), ValueKind::String));
                }

                properties.push(("Optional", bool_value(config_map, "optional"), ValueKind::Boolean));
            }

            if source["downwardAPI"].as_object().is_some() {
                properties.push(("DownwardAPI", true.to_string(), ValueKind::Boolean));
            }

            if let Some(expiration_seconds) = source["serviceAccountToken"]
                .as_object()
                .and_then(|token| token.get("expirationSeconds"))
            {
                properties.push((
                    "TokenExpirationSeconds",
                    value_to_string(expiration_seconds),
                    ValueKind::Numeric,
                ));
            }
        }
    }

    properties
}

fn string_value(source: &Map<String, Value>, key: &str) -> String {
    source.get(key).map(value_to_string).unwrap_or_default()
}

fn bool_value(source: &Map<String, Value>, key: &str) -> String {
    source.get(key).and_then(Value::as_bool).unwrap_or(false).to_string()
}
