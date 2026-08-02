use b4n_config::keys::KeyCommand;
use b4n_kube::{ALL_NAMESPACES, CONTAINERS, EVENTS, NAMESPACES, PODS, Port, ResourceRef, SECRETS, Scope};
use b4n_tui::table::Table;
use b4n_tui::widgets::{ActionItem, ActionsList, ActionsListBuilder, ValidatorKind};
use b4n_tui::{PluginsExt, ResponseEvent};
use std::rc::Rc;

use crate::core::SharedAppData;
use crate::kube::extensions::ActionsListBuilderExt;
use crate::kube::resources::pod::PF_COLUMN_NO;
use crate::ui::views::resources::ResourcesTable;
use crate::ui::widgets::{CommandPalette, StepBuilder};

/// Builds steps required to configure port forward for specified resource container.
pub fn build_port_forward_steps(app_data: &SharedAppData, resource: ResourceRef, list: &[Port]) -> CommandPalette {
    let actions_list = ActionsListBuilder::from_resource_ports(list).build(None);
    CommandPalette::new(Rc::clone(app_data), actions_list, 65)
        .with_header(format!(
            " Add port forward for '{}' container:",
            resource.container.as_deref().unwrap_or_default()
        ))
        .with_prompt("container port")
        .with_validator(ValidatorKind::Number(0, 65_535))
        .with_step(
            StepBuilder::input("")
                .with_validator(ValidatorKind::Number(0, 65_535))
                .with_prompt("local port")
                .with_colors(app_data.borrow().theme.colors.command_palette.clone())
                .with_clipboard(app_data.borrow().get_clipboard())
                .with_copy_previous(true)
                .build(app_data),
        )
        .with_step(
            StepBuilder::input("127.0.0.1")
                .with_validator(ValidatorKind::IpAddr)
                .with_prompt("bind address")
                .with_colors(app_data.borrow().theme.colors.command_palette.clone())
                .with_clipboard(app_data.borrow().get_clipboard())
                .build(app_data),
        )
        .with_response(|v| build_port_forward_response(v, resource))
}

/// Builds command palette that allows to select container's image name.
pub fn build_image_select_palette(app_data: &SharedAppData, highlighted: &str) -> CommandPalette {
    let actions = ActionsListBuilder::from_strings(&app_data.borrow().config.debug_images).build(None);
    CommandPalette::new(Rc::clone(app_data), actions, 65)
        .with_header(" Format: [registry/][namespace/]name[:tag][@digest]")
        .with_prompt("image name")
        .with_validator(ValidatorKind::DockerImage)
        .with_highlighted(highlighted)
        .with_response(|_| ResponseEvent::Action("image_selected"))
}

/// Builds actions for creating new resource.
pub fn build_create_resource_actions(table: &ResourcesTable) -> ActionsList {
    let mut builder = ActionsListBuilder::default()
        .with_menu_action(ActionItem::action("full", "new_full").with_description("get all possible fields for the spec"))
        .with_menu_action(ActionItem::action("minimal", "new_minimal").with_description("get only required fields for the spec"));

    if let Some(name) = table.list.table.get_highlighted_item_name()
        && (table.kind_plural() != NAMESPACES || name != ALL_NAMESPACES)
    {
        builder = builder.with_menu_action(
            ActionItem::action("duplicate", "new_clone")
                .with_description("use the spec of the highlighted resource")
                .with_aliases(["clone"]),
        );
    }

    builder.build(None)
}

/// Builds actions for mouse menu.
pub fn build_mouse_menu_actions(table: &ResourcesTable) -> ActionsList {
    let is_selected = table.list.table.is_anything_selected();
    let highlighted_name = table.list.table.get_highlighted_item_name();
    let is_highlighted = highlighted_name.is_some_and(|n| n != ALL_NAMESPACES);
    let is_containers = table.kind_plural() == CONTAINERS;
    let is_pods = table.kind_plural() == PODS;
    let is_events = table.kind_plural() == EVENTS;

    let copy = if is_selected { "selected" } else { "all" };
    let mut builder = ActionsListBuilder::default()
        .with_menu_action(ActionItem::command_palette())
        .with_menu_action(ActionItem::menu(16, &format!("󰆏 copy ␝{copy}␝"), "copy"));

    if table.kind_plural() != NAMESPACES {
        builder.add_menu_action(ActionItem::menu(100, "󰕍 back", "back"));
    }

    if table.list.table.is_anything_selected() && table.list.table.data.is_deletable {
        let action = ActionItem::menu(14, " delete ␝selected␝", "").with_response(ResponseEvent::AskDeleteResources);
        builder.add_menu_action(action);
    }

    if is_highlighted {
        builder = builder
            .with_menu_action(ActionItem::menu(1, " YAML", "show_yaml"))
            .with_menu_action(ActionItem::menu(5, " describe", "describe"))
            .with_menu_action(ActionItem::menu(15, "󰆏 copy ␝name␝", "copy_name"));

        if table.kind_plural() == SECRETS {
            builder.add_menu_action(ActionItem::menu(2, " YAML ␝decoded␝", "decode_yaml"));
        }

        if is_containers || is_pods {
            builder = builder
                .with_menu_action(ActionItem::menu(3, " logs", "show_logs"))
                .with_menu_action(ActionItem::menu(4, " logs ␝previous␝", "show_plogs"))
                .with_menu_action(ActionItem::menu(6, " attach", "attach"))
                .with_menu_action(ActionItem::menu(7, " shell", "open_shell"))
                .with_menu_action(ActionItem::menu(8, "󱘖 forward port", "port_forward"));

            if is_pods && has_highlighted_item_active_port_forward(table) {
                builder.add_menu_action(ActionItem::menu(9, " stop ␝port forwards␝", "ask_stop_port_forwards"));
            }

            if is_pods {
                builder = builder
                    .with_menu_action(ActionItem::menu(10, " inject container", "inject"))
                    .with_menu_action(ActionItem::menu(11, " download files", "download"))
                    .with_menu_action(ActionItem::menu(11, " upload file", "upload"));
            }
        }

        if table.list.table.data.is_editable {
            builder.add_menu_action(ActionItem::menu(12, " edit", "edit_yaml"));
        }

        if !is_containers && !is_events {
            if table.list.table.data.is_creatable {
                builder.add_menu_action(ActionItem::menu(13, "󰐕 create new", "create"));
            }
            if is_highlighted {
                builder.add_menu_action(ActionItem::menu(98, "󰑏 events", "show_events"));
            }
        }

        if has_involved_object(table) {
            builder.add_menu_action(ActionItem::menu(99, "󰑏 involved object", "show_involved"));
        }
    }

    builder.build(None)
}

/// Builds actions for highlighted resource.
pub fn build_resources_actions(app_data: &SharedAppData, table: &ResourcesTable) -> ActionsList {
    let is_selected = table.list.table.is_anything_selected();
    let is_highlighted = table.list.table.is_anything_highlighted();
    let is_containers = table.kind_plural() == CONTAINERS;
    let is_pods = table.kind_plural() == PODS;
    let is_events = table.kind_plural() == EVENTS;
    let is_deletable = is_selected && table.list.table.data.is_deletable;

    let mut builder = ActionsListBuilder::from_kinds(app_data.borrow().kinds.as_deref())
        .with_resources_actions(!is_containers && is_deletable)
        .with_forwards()
        .with_filter_action("filter")
        .with_pin_filter_action("pin_filter")
        .with_actions(
            app_data
                .borrow()
                .plugins
                .to_actions(table.get_kind().as_str(), is_highlighted, is_selected),
        );

    if table.kind_plural() != NAMESPACES {
        builder.add_action(
            ActionItem::action("back", "back").with_description("returns to the previous view"),
            Some(KeyCommand::NavigateBack),
        );
    }

    if table.scope() == &Scope::Namespaced && !is_containers {
        builder = builder.with_namespace();
    }

    let selected = if is_selected { "selected" } else { "all" };
    builder.add_action(
        ActionItem::action("copy", "copy").with_description(&format!("copies {selected} resources to clipboard")),
        Some(KeyCommand::ContentCopy),
    );

    if !is_containers && !is_events {
        if is_highlighted {
            builder.add_action(
                ActionItem::action("show events", "show_events").with_description("shows events for the selected resource"),
                Some(KeyCommand::EventsShow),
            );
        }

        if table.list.table.data.is_creatable {
            builder.add_action(
                ActionItem::action("create", "create")
                    .with_description("creates new Kubernetes resource")
                    .with_aliases(["new", "add"]),
                Some(KeyCommand::YamlCreate),
            );
        }
    }

    if has_involved_object(table) {
        builder.add_action(
            ActionItem::action("involved object", "show_involved").with_description("navigates to the involved object"),
            Some(KeyCommand::InvolvedObjectShow),
        );
    }

    if is_highlighted {
        builder = add_resource_actions(builder, table, is_containers);
        if is_pods {
            builder = add_ephemeral_container_actions(builder);
            builder = add_file_transfer_actions(builder);
        }
        if is_containers || is_pods {
            builder = add_container_actions(builder);
        }
    }

    builder
        .with_aliases(&app_data.borrow().config.aliases)
        .build(Some(&app_data.borrow().key_bindings))
}

fn add_resource_actions(mut builder: ActionsListBuilder, table: &ResourcesTable, is_containers: bool) -> ActionsListBuilder {
    if table.kind_plural() == SECRETS {
        builder.add_action(
            ActionItem::action("decode", "decode_yaml").with_description("shows decoded YAML of the highlighted secret"),
            Some(KeyCommand::YamlDecode),
        );
    }

    if table.list.table.data.is_editable {
        builder.add_action(
            ActionItem::action("edit YAML", "edit_yaml")
                .with_description("displays YAML and switches to edit mode")
                .with_aliases(["yaml", "yml", "patch"]),
            Some(KeyCommand::YamlEdit),
        );
    }

    builder
        .with_action(
            ActionItem::action("show YAML", "show_yaml")
                .with_description(if is_containers {
                    "shows YAML of the container's resource"
                } else {
                    "shows YAML of the highlighted resource"
                })
                .with_aliases(["yaml", "yml", "view"]),
            Some(KeyCommand::YamlOpen),
        )
        .with_action(
            ActionItem::action("describe", "describe").with_description("shows resource describe view"),
            Some(KeyCommand::DescribeOpen),
        )
}

fn add_ephemeral_container_actions(builder: ActionsListBuilder) -> ActionsListBuilder {
    builder.with_action(
        ActionItem::action("inject container", "inject")
            .with_description("injects ephemeral container")
            .with_aliases(["ephemeral"]),
        Some(KeyCommand::ContainerInject),
    )
}

fn add_file_transfer_actions(builder: ActionsListBuilder) -> ActionsListBuilder {
    builder
        .with_action(
            ActionItem::action("download file", "download")
                .with_description("transfers file from the container")
                .with_aliases(["file", "transfer"]),
            Some(KeyCommand::TransferFrom),
        )
        .with_action(
            ActionItem::action("upload file", "upload")
                .with_description("transfers file to the container")
                .with_aliases(["file", "transfer"]),
            Some(KeyCommand::TransferTo),
        )
}

fn add_container_actions(builder: ActionsListBuilder) -> ActionsListBuilder {
    builder
        .with_action(
            ActionItem::action("show logs", "show_logs")
                .with_description("shows container logs")
                .with_aliases(["logs"]),
            Some(KeyCommand::LogsOpen),
        )
        .with_action(
            ActionItem::action("show previous logs", "show_plogs")
                .with_description("shows container previous logs")
                .with_aliases(["previous"]),
            Some(KeyCommand::PreviousLogsOpen),
        )
        .with_action(
            ActionItem::action("attach", "attach").with_description("attaches to container main process"),
            Some(KeyCommand::ContainerAttach),
        )
        .with_action(
            ActionItem::action("shell", "open_shell").with_description("opens container shell"),
            Some(KeyCommand::ShellOpen),
        )
        .with_action(
            ActionItem::action("forward port", "port_forward")
                .with_description("forwards container port")
                .with_aliases(["port", "pf"]),
            Some(KeyCommand::PortForwardsCreate),
        )
}

fn build_port_forward_response(mut input: Vec<String>, resource: ResourceRef) -> ResponseEvent {
    if input.len() == 3 {
        let container_port = input[0].parse::<u16>().unwrap_or_default();
        let local_port = input[1].parse::<u16>().unwrap_or_default();
        let address = input.pop().unwrap_or_default();
        ResponseEvent::PortForward(resource, container_port, local_port, address)
    } else {
        ResponseEvent::Handled
    }
}

fn has_involved_object(table: &ResourcesTable) -> bool {
    table
        .list
        .table
        .get_highlighted_resource()
        .is_some_and(|res| res.involved_object.is_some())
}

fn has_highlighted_item_active_port_forward(table: &ResourcesTable) -> bool {
    let Some(resource) = table.list.table.get_highlighted_resource().and_then(|r| r.data.as_ref()) else {
        return false;
    };

    resource.extra_values.len() > PF_COLUMN_NO && resource.extra_values[PF_COLUMN_NO].raw_text().is_some_and(|t| !t.is_empty())
}
