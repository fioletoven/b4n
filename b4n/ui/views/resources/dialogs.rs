use b4n_config::{PluginInputType, PluginRef};
use b4n_kube::plugins::PluginContext;
use b4n_kube::{ResourceRef, ResourceTag};
use b4n_tui::widgets::{Button, CheckBox, Dialog, Selector, TextBox, ValidatorKind};
use b4n_tui::{EphemeralContainer, ResponseEvent};
use ratatui::layout::Position;
use std::collections::HashMap;

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::ui::views::common;
use crate::ui::views::resources::ResourcesTable;

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
    let inputs = plugin.inputs.then(|| app_data.get_plugin_inputs(&plugin.id)).flatten();
    let message = if plugin.inputs {
        format!("Provide additional data for '{}':", plugin.name)
    } else {
        format!("Are you sure you want to run '{}'?", plugin.name)
    };

    let mut dialog = Dialog::new(
        message,
        vec![
            Button::new("Run", ResponseEvent::PluginAction(plugin), colors.modal.btn_delete.clone()),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
        ],
    )
    .with_colors(colors.modal.text)
    .with_highlighted_position(position);

    if let Some(inputs) = inputs {
        for (idx, input) in inputs.into_iter().enumerate() {
            match input.kind {
                PluginInputType::CheckBox => {
                    let colors = colors.modal.checkbox.clone();
                    let is_checked = input.options.len() == 2 && input.value.is_some_and(|v| input.options[1] == v);
                    dialog = dialog.with_checkboxes(vec![CheckBox::new(idx, input.label, is_checked, colors)]);
                },
                PluginInputType::TextBox => {
                    let colors = colors.modal.textbox.clone();
                    let mut tb = TextBox::new(idx, input.label, 50, colors)
                        .with_value(input.value.unwrap_or_default())
                        .with_clipboard(app_data.borrow().get_clipboard());
                    if input.required {
                        tb = tb.with_validator(ValidatorKind::Required);
                    }
                    dialog = dialog.with_textboxes(vec![tb]);
                },
                PluginInputType::Select => {
                    let colors = &colors.modal.selector;
                    let selected = input.value.and_then(|v| input.options.iter().position(|i| *i == v));
                    dialog = dialog.with_selectors(vec![
                        Selector::new(idx, input.label, &input.options, colors).with_selected(selected.unwrap_or(0)),
                    ]);
                },
            }
        }
    }

    dialog
}

/// Builds plugin context enriched by the dialog inputs.
pub fn build_plugin_context(
    app_data: &SharedAppData,
    table: &ResourcesTable,
    dialog: &Dialog,
    plugin: &PluginRef,
) -> PluginContext {
    let mut resources = Vec::new();
    let mut values = Vec::new();
    let mut inputs = HashMap::new();

    if plugin.highlighted {
        if let Some(resource) = table.get_resource_ref(false) {
            resources.push(resource);
        }

        if let Some(result) = table.get_column_values() {
            values.push(result);
        }
    }

    if plugin.selected {
        resources.append(&mut table.get_selected_resources_ref(false));
        values.append(&mut table.get_selected_column_values());
    }

    if plugin.inputs
        && let Some(config) = app_data.get_plugin_inputs(&plugin.id)
    {
        for (idx, mut input) in config.into_iter().enumerate() {
            let value = match input.kind {
                PluginInputType::CheckBox => dialog.checkbox(idx).map(|cb| match input.options.as_slice() {
                    [_, _] => input.options.remove(usize::from(cb.is_checked)),
                    _ => cb.is_checked.to_string(),
                }),
                PluginInputType::TextBox => dialog.textbox(idx).map(|tb| tb.value().to_owned()),
                PluginInputType::Select => dialog.selector(idx).map(|sel| sel.selected().to_owned()),
            };

            if let Some(value) = value {
                inputs.insert(input.name.to_ascii_uppercase(), value);
            }
        }
    }

    let app_data = app_data.borrow();
    PluginContext {
        kubeconfig: app_data.history.kube_config_path().map(String::from).unwrap_or_default(),
        context: app_data.current.context.clone(),
        kind: app_data.current.resource.kind.clone(),
        namespace: app_data.current.namespace.clone(),
        resources,
        columns: table.get_column_names(),
        values,
        inputs,
    }
}

/// Builds modal dialog to inject ephemeral container.
pub fn new_inject_container_dialog(app_data: &SharedAppData, resource: &ResourceRef, tags: &[ResourceTag]) -> Dialog {
    let colors = &app_data.borrow().theme.colors.modal;
    let image = &app_data.borrow().config.debug_images.first().cloned().unwrap_or_default();
    let except_names = common::get_all_containers_from_resource_tags(tags);
    let mut target_names = common::get_target_containers_from_resource_tags(tags);
    target_names.insert(0, "--none--".to_string());
    let idx_to_select = usize::from(target_names.len() > 1);

    Dialog::new(
        format!(
            "Inject ephemeral container into '{}':",
            resource.name.as_deref().unwrap_or_default()
        ),
        vec![
            Button::new("Inject Container", ResponseEvent::Action("inject"), colors.btn_accent.clone()),
            Button::new("Cancel", ResponseEvent::Cancelled, colors.btn_cancel.clone()),
        ],
    )
    .with_colors(colors.text)
    .with_textboxes(vec![
        TextBox::new(0, "Name:   ", 40, colors.textbox.clone())
            .with_value("debug")
            .with_clipboard(app_data.borrow().get_clipboard())
            .with_validator(ValidatorKind::Required)
            .with_validator(ValidatorKind::StringExcept(except_names))
            .with_validator(ValidatorKind::DnsLabel),
        TextBox::new(1, "Image:  ", 40, colors.textbox.clone())
            .with_value(image)
            .with_clipboard(app_data.borrow().get_clipboard())
            .with_validator(ValidatorKind::DockerImage)
            .with_button("  ", "select_image"),
        TextBox::new(2, "Command:", 40, colors.textbox.clone())
            .with_clipboard(app_data.borrow().get_clipboard())
            .with_validator(ValidatorKind::ShellCommand),
    ])
    .with_selectors(vec![
        Selector::new(0, "Target: ", &target_names, &colors.selector).with_selected(idx_to_select),
    ])
}

/// Returns new [`ResponseEvent::InjectContainer`] response built from the properties set in the modal dialog.
pub fn build_inject_container_response(modal: &Dialog, resource: ResourceRef) -> ResponseEvent {
    fn get_textbox_value(modal: &Dialog, idx: usize) -> String {
        modal.textbox(idx).map_or_else(String::new, |tb| tb.value().to_string())
    }

    let container = EphemeralContainer {
        name: get_textbox_value(modal, 0),
        image: get_textbox_value(modal, 1),
        command: get_textbox_value(modal, 2),
        target: modal
            .selector(0)
            .map(Selector::selected)
            .filter(|&s| s != "--none--")
            .map(String::from),
    };
    ResponseEvent::InjectContainer(resource, container)
}
