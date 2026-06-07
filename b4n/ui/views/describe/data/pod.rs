use b4n_kube::{CONTAINERS, InitData, ObserverResult, ResourceRef};
use b4n_tui::table::{Column, Table, ViewType};
use k8s_openapi::serde_json::{Map, Value};
use kube::ResourceExt;
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::core::SharedAppData;
use crate::kube::resources::{ColumnsLayout, ResourceItem, ResourcesList};
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::utils::{ValueKind, header, map_join, map_to_string, uppercase_first_letter, value_to_string};
use crate::ui::widgets::table::{BasicRow, BasicTable, Cell};
use crate::ui::{presentation::ListViewer, presentation::StyledLine, views::describe::data::SectionData};

/// Returns additional describe sections for `pod` resource.
pub fn create_additional_sections(_resource: &ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let colors = &app_data.borrow().theme.colors.syntax.describe;

    vec![
        SectionData::Text(Vec::new()),
        SectionData::Text(vec![StyledLine::default(), header(colors, "Containers", 0)]),
        SectionData::Resources(Box::new(create_containers_table(app_data))),
        SectionData::Text(Vec::new()),
        SectionData::Text(vec![StyledLine::default(), header(colors, "Tolerations", 0)]),
        SectionData::List(Box::new(create_tolerations_table(app_data))),
    ]
}

/// Updates additional describe sections for `pod` resource.
pub fn update_additional_sections(
    resource: &ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 6 {
        return;
    }

    update_data_section(app_data, object, &mut sections[0]);
    update_containers_section(resource, object, &mut sections[2]);
    update_volume_section(app_data, object, &mut sections[3]);
    update_tolerations_section(object, &mut sections[5]);
}

fn create_containers_table(app_data: &SharedAppData) -> ListViewer<ResourcesList> {
    let mut table = ListViewer::new(
        Rc::clone(app_data),
        ResourcesList::default()
            .with_columns_layout(ColumnsLayout::Compact)
            .with_focus(false),
        ViewType::Compact,
    )
    .with_no_border()
    .with_focus(false);

    table.table.table.limit_offset(false);
    table
}

fn create_tolerations_table(app_data: &SharedAppData) -> ListViewer<BasicTable> {
    let mut table = ListViewer::new(
        Rc::clone(app_data),
        BasicTable::new(
            Column::bound("KEY", 10, 50, false),
            Box::new([
                Column::fixed("OPERATOR", 10, false),
                Column::bound("VALUE", 6, 30, false),
                Column::fixed("SECONDS", 8, true),
                Column::bound("EFFECT", 6, 20, false),
            ]),
            &['K', 'O', 'V', 'E', 'S'],
        )
        .with_focus(false),
        ViewType::Compact,
    )
    .with_no_border()
    .with_focus(false);

    table.table.table.limit_offset(false);
    table
}

fn update_tolerations_section(object: &DynamicObject, section: &mut SectionData) {
    if let SectionData::List(list) = section
        && let Some(tolerations) = object.data["spec"]["tolerations"].as_array()
    {
        list.table.clear();
        for item in tolerations {
            let key = item["key"].as_str().unwrap_or_default();
            let row = BasicRow::new(
                format!("_{key}_"),
                key,
                Box::new([
                    item["operator"].as_str().unwrap_or("Equal").into(),
                    item["value"].as_str().unwrap_or_default().into(),
                    Cell::integer(item["tolerationSeconds"].as_i64(), 6),
                    item["effect"].as_str().unwrap_or_default().into(),
                ]),
            );
            list.table.update(row, false);
        }
    }
}

fn update_data_section(app_data: &SharedAppData, object: &DynamicObject, section: &mut SectionData) {
    let SectionData::Text(lines) = section else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let mut builder = TextSectionBuilder::new(colors, lines);

    add_networking_section(&mut builder, object);
    add_scheduling_section(&mut builder, object);
    add_runtime_section(&mut builder, object);
}

fn add_networking_section(builder: &mut TextSectionBuilder, object: &DynamicObject) {
    builder.start_section("Networking", 0, 2, Some(16));

    let status = &object.data["status"];
    builder.add_str("Pod IP", status["podIP"].as_str());
    builder.add_str(
        "Pod IPs",
        map_join(status["podIPs"].as_array(), |i| value_to_string(&i["ip"])),
    );
    builder.add_str("Host IP", status["hostIP"].as_str());
    builder.add_str(
        "Host IPs",
        map_join(status["hostIPs"].as_array(), |i| value_to_string(&i["ip"])),
    );

    let spec = &object.data["spec"];
    builder.add_bool("Host Network", spec["hostNetwork"].as_bool());
    builder.add_str("DNS Policy", spec["dnsPolicy"].as_str());
    builder.add_str(
        "DNS Nameservers",
        map_join(spec["dnsConfig"]["nameservers"].as_array(), value_to_string),
    );
    builder.add_str("DNS Options", dns_options(spec["dnsConfig"]["options"].as_array()));
    builder.add_str(
        "DNS Searches",
        map_join(spec["dnsConfig"]["searches"].as_array(), value_to_string),
    );
}

fn dns_options(values: Option<&Vec<Value>>) -> Option<String> {
    map_join(values, |item| {
        Some(format!("{}={}", item["name"].as_str()?, item["value"].as_str()?))
    })
}

fn add_scheduling_section(builder: &mut TextSectionBuilder, object: &DynamicObject) {
    builder.start_section("Scheduling", 0, 2, Some(28));

    let spec = &object.data["spec"];
    builder.add_str("Node", spec["nodeName"].as_str());
    builder.add_str("Nominated Node", object.data["status"]["nominatedNodeName"].as_str());
    builder.add_str("Scheduler", spec["schedulerName"].as_str());
    builder.add_num("Priority", spec["priority"].as_i64().map(|v| v.to_string()));
    builder.add_str("Priority Class", spec["priorityClassName"].as_str());
    builder.add_str("Preemption Policy", spec["preemptionPolicy"].as_str());
    builder.add_str("Node Selector", map_to_string(spec["nodeSelector"].as_object()));
    builder.add_str(
        "Scheduling Gates",
        map_join(spec["schedulingGates"].as_array(), |i| value_to_string(&i["name"])),
    );
    builder.add_str(
        "Readiness Gates",
        map_join(spec["readinessGates"].as_array(), |i| value_to_string(&i["conditionType"])),
    );
    builder.add_str(
        "Topology Spread Constraints",
        topology_spread_constraints(spec["topologySpreadConstraints"].as_array()),
    );
}

fn topology_spread_constraints(values: Option<&Vec<Value>>) -> Option<String> {
    let items = values?
        .iter()
        .map(|item| {
            let topology_key = item["topologyKey"].as_str().unwrap_or_default();
            let when_unsatisfiable = item["whenUnsatisfiable"].as_str().unwrap_or_default();
            let skew = item["maxSkew"].as_i64().map(|value| value.to_string()).unwrap_or_default();
            format!("{topology_key} / {when_unsatisfiable} / skew {skew}")
        })
        .filter(|value| !value.trim_matches('/').trim().is_empty())
        .collect::<Vec<_>>();
    (!items.is_empty()).then_some(items.join(", "))
}

fn add_runtime_section(builder: &mut TextSectionBuilder, object: &DynamicObject) {
    builder.start_section("Runtime", 0, 2, Some(24));

    let spec = &object.data["spec"];
    builder.add_str("Service Account", spec["serviceAccountName"].as_str());
    builder.add_str("Runtime Class", spec["runtimeClassName"].as_str());
    builder.add_str("Restart Policy", spec["restartPolicy"].as_str());
    builder.add_str("QoS Class", object.data["status"]["qosClass"].as_str());
    builder.add_str(
        "Image Pull Secrets",
        map_join(spec["imagePullSecrets"].as_array(), |i| value_to_string(&i["name"])),
    );
    builder.add_bool("Enable Service Links", spec["enableServiceLinks"].as_bool());
    builder.add_bool("Share Process Namespace", spec["shareProcessNamespace"].as_bool());
    builder.add_bool("Host PID", spec["hostPID"].as_bool());
    builder.add_bool("Host IPC", spec["hostIPC"].as_bool());
    builder.add_str("Security Context", pod_security_context(spec["securityContext"].as_object()));
}

fn pod_security_context(values: Option<&Map<String, Value>>) -> Option<String> {
    let values = values?;
    let mut items = Vec::new();

    if let Some(run_as_user) = values.get("runAsUser").and_then(Value::as_i64) {
        items.push(format!("runAsUser={run_as_user}"));
    }
    if let Some(run_as_group) = values.get("runAsGroup").and_then(Value::as_i64) {
        items.push(format!("runAsGroup={run_as_group}"));
    }
    if let Some(run_as_non_root) = values.get("runAsNonRoot").and_then(Value::as_bool) {
        items.push(format!("runAsNonRoot={run_as_non_root}"));
    }
    if let Some(fs_group) = values.get("fsGroup").and_then(Value::as_i64) {
        items.push(format!("fsGroup={fs_group}"));
    }
    if let Some(supplemental_groups) = values.get("supplementalGroups").and_then(Value::as_array) {
        let groups = supplemental_groups
            .iter()
            .filter_map(Value::as_i64)
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        if !groups.is_empty() {
            items.push(format!("supplementalGroups={}", groups.join("/")));
        }
    }
    if let Some(seccomp_type) = values
        .get("seccompProfile")
        .and_then(Value::as_object)
        .and_then(|profile| profile.get("type"))
        .and_then(Value::as_str)
    {
        items.push(format!("seccomp={seccomp_type}"));
    }

    (!items.is_empty()).then_some(items.join(", "))
}

fn update_containers_section(resource: &ResourceRef, object: &DynamicObject, section: &mut SectionData) {
    let SectionData::Resources(list) = section else {
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

fn update_volume_section(app_data: &SharedAppData, object: &DynamicObject, section: &mut SectionData) {
    let SectionData::Text(lines) = section else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let mut builder = TextSectionBuilder::new(colors, lines);
    builder.start_section("Volumes", 0, 2, None);

    let Some(volumes) = object.data["spec"]["volumes"].as_array() else {
        builder.add_none();
        return;
    };

    if volumes.is_empty() {
        builder.add_none();
        return;
    }

    for volume in volumes {
        add_volume(&mut builder, volume);
    }
}

fn add_volume(builder: &mut TextSectionBuilder, volume: &Value) {
    let Some(name) = volume["name"].as_str() else {
        return;
    };

    let properties = get_volume_properties(volume);
    let width = properties.iter().map(|(key, _, _)| key.len()).max().unwrap_or_default() + 1;

    builder.sub_section(name, 2, 4, Some(width));
    for (key, value, kind) in properties {
        builder.add_line(key, value, kind);
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
        ("ReadOnly", string_value(pvc, "readOnly"), ValueKind::Boolean),
    ]
}

fn secret_properties(secret: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "Secret".to_owned(), ValueKind::String),
        ("SecretName", string_value(secret, "secretName"), ValueKind::String),
        ("Optional", string_value(secret, "optional"), ValueKind::Boolean),
    ]
}

fn config_map_properties(config_map: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "ConfigMap".to_owned(), ValueKind::String),
        ("Name", string_value(config_map, "name"), ValueKind::String),
        ("Optional", string_value(config_map, "optional"), ValueKind::Boolean),
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
    let limit = empty_dir.get("sizeLimit").and_then(value_to_string);
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
        ("ReadOnly", string_value(nfs, "readOnly"), ValueKind::Boolean),
    ]
}

fn csi_properties(csi: &Map<String, Value>) -> Vec<TypedProperty> {
    vec![
        ("Type", "CSI".to_owned(), ValueKind::String),
        ("Driver", string_value(csi, "driver"), ValueKind::String),
        ("FSType", string_value(csi, "fsType"), ValueKind::String),
        ("ReadOnly", string_value(csi, "readOnly"), ValueKind::Boolean),
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
        .and_then(value_to_string);
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
                    properties.push(("SecretName", value_to_string(name).unwrap_or_default(), ValueKind::String));
                }

                properties.push(("Optional", string_value(secret, "optional"), ValueKind::Boolean));
            }

            if let Some(config_map) = source["configMap"].as_object() {
                if let Some(name) = config_map.get("name") {
                    properties.push(("ConfigMapName", value_to_string(name).unwrap_or_default(), ValueKind::String));
                }

                properties.push(("Optional", string_value(config_map, "optional"), ValueKind::Boolean));
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
                    value_to_string(expiration_seconds).unwrap_or_default(),
                    ValueKind::Numeric,
                ));
            }
        }
    }

    properties
}

fn string_value(source: &Map<String, Value>, key: &str) -> String {
    source.get(key).and_then(value_to_string).unwrap_or_default()
}
