use b4n_common::{truncate, truncate_left};
use b4n_config::PluginRef;
use b4n_kube::ResourceRef;
use b4n_tui::widgets::{Button, CheckBox, Dialog, Selector, TextBox};
use b4n_tui::{EphemeralContainer, ResponseEvent};
use ratatui::layout::Position;

use crate::core::SharedAppData;

/// Creates new resource delete confirmation dialog.
pub fn new_delete_dialog(app_data: &SharedAppData, position: Option<Position>) -> Dialog {
    let colors = &app_data.borrow().theme.colors;
    Dialog::new(
        "Are you sure you want to delete the selected resources?".to_owned(),
        vec![
            Button::new("Delete", ResponseEvent::Action("delete"), &colors.modal.btn_delete),
            Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
        ],
    )
    .with_colors(colors.modal.text)
    .with_checkboxes(vec![
        CheckBox::new(0, "Terminate immediately", false, &colors.modal.checkbox),
        CheckBox::new(1, "Remove finalizers before deletion", false, &colors.modal.checkbox),
    ])
    .with_selectors(vec![Selector::new(
        0,
        "Propagation policy:",
        &["None", "Background", "Foreground", "Orphan"],
        &colors.modal.selector,
    )])
    .with_highlighted_position(position)
}

/// Creates new stop port forwarding rules dialog.
pub fn new_stop_port_forwards_dialog(app_data: &SharedAppData, position: Option<Position>, resource: &str) -> Dialog {
    let colors = &app_data.borrow().theme.colors;
    Dialog::new(
        format!("Are you sure you want to stop all port forwarding rules for '{resource}'?"),
        vec![
            Button::new("Stop", ResponseEvent::Action("stop_port_forwards"), &colors.modal.btn_delete),
            Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
        ],
    )
    .with_colors(colors.modal.text)
    .with_highlighted_position(position)
}

/// Creates new dialog for run plugin confirmation.
pub fn new_run_plugin_dialog(app_data: &SharedAppData, position: Option<Position>, plugin: PluginRef) -> Dialog {
    let colors = &app_data.borrow().theme.colors;
    Dialog::new(
        format!("Are you sure you want to run '{}'?", plugin.name),
        vec![
            Button::new("Run", ResponseEvent::PluginAction(plugin), &colors.modal.btn_delete),
            Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
        ],
    )
    .with_colors(colors.modal.text)
    .with_highlighted_position(position)
}

/// Creates new inject container confirmation dialog.
pub fn new_inject_container_dialog(
    app_data: &SharedAppData,
    position: Option<Position>,
    resource: ResourceRef,
    container: EphemeralContainer,
) -> Dialog {
    let len = 38;

    let colors = &app_data.borrow().theme.colors;
    let msg = "Are you sure you want to inject ephemeral container?";
    let image_tr = if container.image.chars().count() > len { "…" } else { "" };
    let command_tr = if container.command.chars().count() > len { "…" } else { "" };

    let msg = format!(
        "{msg}\n\n    name:    {}\n    image:   {}{}\n    command: {}{}\n    target:  {}",
        container.name,
        image_tr,
        truncate_left(&container.image, len),
        truncate(&container.command, len),
        command_tr,
        container.target.as_deref().unwrap_or_default()
    );

    Dialog::new(
        msg,
        vec![
            Button::new(
                "Inject Container",
                ResponseEvent::InjectContainer(resource, container),
                &colors.modal.btn_accent,
            ),
            Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
        ],
    )
    .with_colors(colors.modal.text)
    .with_highlighted_position(position)
}

/// Creates new transfer files dialog.
pub fn new_transfer_dialog(is_download: bool, app_data: &SharedAppData, position: Option<Position>) -> Dialog {
    let colors = &app_data.borrow().theme.colors;
    Dialog::new(
        get_transfer_dialog_title(is_download),
        vec![
            Button::new("Transfer", ResponseEvent::Action("transfer_file"), &colors.modal.btn_accent),
            Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
        ],
    )
    .with_colors(colors.modal.text)
    .with_checkboxes(vec![CheckBox::new(0, "Download", is_download, &colors.modal.checkbox)])
    .with_textboxes(vec![
        TextBox::new(0, "From:     ", 40, colors.modal.textbox.clone()).with_button("  ", "file_from"),
        TextBox::new(0, "To:       ", 40, colors.modal.textbox.clone()).with_button("  ", "file_to"),
    ])
    .with_selectors(vec![Selector::new(
        0,
        "Container:",
        &["first", "second", "third"],
        &colors.modal.selector,
    )])
    .with_highlighted_position(position)
    .with_on_change(|message, controls| {
        let is_checked = controls.checkbox(0).is_some_and(|c| c.is_checked);
        let is_download = message.starts_with("Download");
        let is_ok = (is_checked && is_download) || (!is_checked && !is_download);
        if !is_ok {
            *message = get_transfer_dialog_title(is_checked);
        }
    })
}

fn get_transfer_dialog_title(is_download: bool) -> String {
    if is_download {
        "Download file from the specified container:".to_owned()
    } else {
        "Upload file to the specified container:".to_owned()
    }
}
