use b4n_kube::ResourceRef;
use b4n_tui::table::{Column, Table, ViewType};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::core::SharedAppData;
use crate::ui::presentation::{ListViewer, StyledLine};
use crate::ui::views::describe::builder::TextSectionBuilder;
use crate::ui::views::describe::data::SectionData;
use crate::ui::views::describe::utils::{header, map_join, map_to_string, value_to_string};
use crate::ui::widgets::table::{BasicRow, BasicTable, Cell};

/// Returns additional describe sections for `service` resource.
pub fn create_additional_sections(_resource: &ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let colors = &app_data.borrow().theme.colors.syntax.describe;

    vec![
        SectionData::Text(Vec::new(), 0),
        SectionData::Text(vec![StyledLine::default(), header(colors, "Ports", 0)], 0),
        SectionData::List(Box::new(create_ports_table(app_data)), 0),
    ]
}

/// Updates additional describe sections for `service` resource.
pub fn update_additional_sections(
    _resource: &ResourceRef,
    app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 3 {
        return;
    }

    update_networking_section(app_data, object, &mut sections[0]);
    update_ports_table(object, &mut sections[2]);
}

fn create_ports_table(app_data: &SharedAppData) -> ListViewer<BasicTable> {
    let mut table = ListViewer::new(
        Rc::clone(app_data),
        BasicTable::new(
            Column::fixed("PROTOCOL", 10, false),
            Box::new([
                Column::bound("NAME", 10, 30, false),
                Column::fixed("PORT", 8, false),
                Column::fixed("TARGET PORT", 12, false),
                Column::fixed("NODE PORT", 10, false),
                Column::bound("APP PROTOCOL", 14, 25, false),
            ]),
            &['R', 'N', 'P', 'T', 'O', 'A'],
        )
        .with_focus(false),
        ViewType::Compact,
    )
    .with_no_border()
    .with_focus(false);

    table.table.table.limit_offset(false);
    table.table.sort(2, false);

    table
}

fn update_ports_table(object: &DynamicObject, section: &mut SectionData) {
    if let SectionData::List(list, _) = section
        && let Some(ports) = object.data["spec"]["ports"].as_array()
    {
        list.table.clear();
        for port in ports {
            let name = port["name"]
                .as_str()
                .map_or_else(|| format!("{}", port["port"].as_i64().unwrap_or_default()), String::from);
            let uid = format!("_{name}_");
            let row = BasicRow::new(
                uid,
                port["protocol"].as_str().unwrap_or_default(),
                Box::new([
                    name.into(),
                    Cell::integer(port["port"].as_i64(), 6),
                    port.get("targetPort").and_then(value_to_string).into(),
                    Cell::integer(port["nodePort"].as_i64(), 6),
                    port["appProtocol"].as_str().unwrap_or_default().into(),
                ]),
            );
            list.table.update(row, false);
        }
    }
}

fn update_networking_section(app_data: &SharedAppData, object: &DynamicObject, section: &mut SectionData) {
    let SectionData::Text(lines, _) = section else {
        return;
    };

    lines.clear();

    let colors = &app_data.borrow().theme.colors.syntax.describe;
    let mut builder = TextSectionBuilder::new(colors, lines);

    let spec = &object.data["spec"];
    builder.start_section("Networking", 0, 2, Some(24));
    builder.add_str("Type", spec["type"].as_str());
    builder.add_str("Cluster IP", spec["clusterIP"].as_str());
    builder.add_str("Cluster IPs", spec["clusterIPs"].as_str());
    builder.add_str("External Name", spec["externalName"].as_str());
    builder.add_str("External IPs", map_join(spec["externalIPs"].as_array(), value_to_string));
    builder.add_str("Selector", map_to_string(spec["selector"].as_object()));
    builder.add_str("Session Affinity", spec["sessionAffinity"].as_str());
    builder.add_str("Internal Traffic Policy", spec["internalTrafficPolicy"].as_str());
    builder.add_str("External Traffic Policy", spec["externalTrafficPolicy"].as_str());
    builder.add_str("Traffic Distribution", spec["trafficDistribution"].as_str());
    builder.add_str("IP Families", map_join(spec["ipFamilies"].as_array(), value_to_string));
    builder.add_str("IP Family Policy", spec["ipFamilyPolicy"].as_str());
    builder.add_str("Load Balancer Class", spec["loadBalancerClass"].as_str());
    builder.add_str(
        "Load Balancer Ingress",
        map_join(object.data["status"]["loadBalancer"]["ingress"].as_array(), |item| {
            item["ip"].as_str().or_else(|| item["hostname"].as_str()).map(String::from)
        }),
    );
    builder.add_num(
        "Health Check Node Port",
        spec["healthCheckNodePort"].as_i64().map(|value| value.to_string()),
    );
}
