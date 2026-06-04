use b4n_kube::ResourceRef;
use b4n_tui::table::{Column, Table, ViewType};
use kube::api::DynamicObject;
use std::rc::Rc;

use crate::core::SharedAppData;
use crate::ui::presentation::{ListViewer, StyledLine};
use crate::ui::views::describe::data::SectionData;
use crate::ui::views::describe::utils::{header, value_to_string};
use crate::ui::widgets::table::{BasicRow, BasicTable, Cell};

/// Returns additional describe sections for `service` resource.
pub fn create_additional_sections(_resource: &ResourceRef, app_data: &SharedAppData) -> Vec<SectionData> {
    let mut viewer = ListViewer::new(
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
    viewer.table.table.limit_offset(false);
    viewer.table.sort(2, false);

    let colors = &app_data.borrow().theme.colors.syntax.describe;

    vec![
        SectionData::Text(vec![StyledLine::default(), header(colors, "Ports")]),
        SectionData::List(Box::new(viewer)),
    ]
}

/// Updates additional describe sections for `service` resource.
pub fn update_additional_sections(
    _resource: &ResourceRef,
    _app_data: &SharedAppData,
    object: &DynamicObject,
    sections: &mut [SectionData],
) {
    if sections.len() != 2 {
        return;
    }

    if let SectionData::List(list) = &mut sections[1]
        && let Some(ports) = object.data["spec"]["ports"].as_array()
    {
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
                    port.get("targetPort").map(value_to_string).into(),
                    Cell::integer(port["nodePort"].as_i64(), 6),
                    port["appProtocol"].as_str().unwrap_or_default().into(),
                ]),
            );
            list.table.update(row, false);
        }
    }
}
