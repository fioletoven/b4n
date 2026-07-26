use b4n_common::{truncate, truncate_left};
use b4n_config::PluginRef;
use b4n_config::themes::TextBoxModalColors;
use b4n_kube::files::TransferContext;
use b4n_kube::{ResourceRef, ResourceTag};
use b4n_tui::widgets::{Button, CheckBox, Dialog, Selector, TextBox};
use b4n_tui::{EphemeralContainer, ResponseEvent};
use ratatui::layout::Position;

use crate::core::SharedAppData;
use crate::ui::views::resources::utils;
use crate::ui::widgets::FileSelector;

/// Creates new resource delete confirmation dialog.
pub fn new_delete_dialog(app_data: &SharedAppData, position: Option<Position>) -> Dialog {
    let colors = &app_data.borrow().theme.colors;
    Dialog::new(
        "Are you sure you want to delete the selected resources?".to_owned(),
        vec![
            Button::new("Delete", ResponseEvent::Action("delete"), colors.modal.btn_delete.clone()),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
        ],
    )
    .with_colors(colors.modal.text)
    .with_checkboxes(vec![
        CheckBox::new(0, "Terminate immediately", false, colors.modal.checkbox.clone()),
        CheckBox::new(1, "Remove finalizers before deletion", false, colors.modal.checkbox.clone()),
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
            Button::new(
                "Stop",
                ResponseEvent::Action("stop_port_forwards"),
                colors.modal.btn_delete.clone(),
            ),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
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
            Button::new("Run", ResponseEvent::PluginAction(plugin), colors.modal.btn_delete.clone()),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
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
                colors.modal.btn_accent.clone(),
            ),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
        ],
    )
    .with_colors(colors.modal.text)
    .with_highlighted_position(position)
}

/// Creates new transfer files dialog.
pub fn new_transfer_dialog(
    is_download: bool,
    app_data: &SharedAppData,
    tags: &[ResourceTag],
    position: Option<Position>,
) -> Dialog {
    let colors = &app_data.borrow().theme.colors;
    Dialog::new(
        get_transfer_dialog_title(is_download),
        vec![
            Button::new(
                "Transfer",
                ResponseEvent::Action("transfer_file"),
                colors.modal.btn_accent.clone(),
            ),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
        ],
    )
    .with_colors(colors.modal.text)
    .with_checkboxes(vec![CheckBox::new(0, "Download", is_download, colors.modal.checkbox.clone())])
    .with_textboxes(get_transfer_dialog_textboxes(app_data, is_download, &colors.modal.textbox))
    .with_selectors(vec![Selector::new(
        0,
        "Container:",
        &utils::get_all_containers_from_resource_tags(tags),
        &colors.modal.selector,
    )])
    .with_highlighted_position(position)
    .with_on_change(|message, controls| {
        let is_checked = controls.checkbox(0).is_some_and(|c| c.is_checked);
        let is_download = message.starts_with("Download");
        let is_ok = (is_checked && is_download) || (!is_checked && !is_download);
        if !is_ok {
            *message = get_transfer_dialog_title(is_checked);
            controls.controls_mut().swap(1, 2);

            if let Some(textbox) = controls.controls_mut()[1].as_textbox_mut() {
                setup_transfer_dialog_textbox(textbox, is_download, true);
            }

            if let Some(textbox) = controls.controls_mut()[2].as_textbox_mut() {
                setup_transfer_dialog_textbox(textbox, !is_download, false);
            }
        }
    })
}

/// Returns file transfer context for background task.
pub fn get_transfer_dialog_context(dialog: &Dialog) -> Option<TransferContext> {
    let is_download = dialog.checkbox(0).is_some_and(|cb| cb.is_checked);
    let container = dialog.selector(0).map(|s| s.selected().to_owned())?;
    let first = dialog.textbox(0).map(|tb| tb.value().to_owned())?;
    let second = dialog.textbox(1).map(|tb| tb.value().to_owned())?;

    if is_download {
        Some(TransferContext::download(second, first, container))
    } else {
        Some(TransferContext::upload(first, second, container))
    }
}

/// Updates transfer dialog textboxes with the selected path from a file picker.
pub fn update_transfer_dialog_paths(dialog: &mut Dialog, file_picker: &FileSelector) {
    let (path, exists) = file_picker.selected_path();
    let is_download = dialog.checkbox(0).is_some_and(|cb| cb.is_checked);
    if is_download || exists {
        if let Some(textbox) = dialog.textbox_mut(0)
            && let Ok(path) = path.clone().into_os_string().into_string()
        {
            textbox.set_value(path);
        }

        if let Some(textbox) = dialog.textbox_mut(1)
            && textbox.value().is_empty()
            && let Some(filename) = path.file_name()
            && let Some(filename_str) = filename.to_str()
        {
            textbox.set_value(format!("~/{}", filename_str));
        }
    }
}

fn get_transfer_dialog_title(is_download: bool) -> String {
    if is_download {
        "Download file from the specified container:".to_owned()
    } else {
        "Upload file to the specified container:".to_owned()
    }
}

fn get_transfer_dialog_textboxes(app_data: &SharedAppData, is_download: bool, colors: &TextBoxModalColors) -> Vec<TextBox> {
    fn get_textbox(app_data: &SharedAppData, is_download: bool, is_first: bool, colors: &TextBoxModalColors) -> TextBox {
        let mut textbox = TextBox::new(usize::from(is_download), "", 40, colors.clone())
            .with_button("  ", "select_file")
            .with_clipboard(app_data.borrow().get_clipboard());
        setup_transfer_dialog_textbox(&mut textbox, !is_download, is_first);
        textbox
    }

    vec![
        get_textbox(app_data, is_download, true, colors),
        get_textbox(app_data, !is_download, false, colors),
    ]
}

fn setup_transfer_dialog_textbox(textbox: &mut TextBox, has_button: bool, is_first: bool) {
    textbox.show_button(has_button);
    textbox.set_caption(if is_first { "From:     " } else { "To:       " });
}
