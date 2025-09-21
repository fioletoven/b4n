use delegate::delegate;
use kube::{config::NamedContext, discovery::Scope};
use ratatui::{Frame, layout::Rect};
use std::{collections::HashMap, rc::Rc};

use crate::{
    core::{SharedAppData, SharedAppDataExt, SharedBgWorker},
    kubernetes::{
        Kind, Namespace, ResourceRef,
        resources::{CONTAINERS, NODES, PODS, Port, ResourceItem, SECRETS, node, pod},
        watchers::{ObserverResult, SharedStatistics},
    },
    ui::{
        KeyCommand, MouseEventKind, Responsive, Table, ViewType,
        tui::{ResponseEvent, TuiEvent},
        widgets::{ActionItem, ActionsListBuilder, Button, CommandPalette, Dialog, Filter, StepBuilder, ValidatorKind},
    },
};

use super::ResourcesTable;

/// Resources view (main view) for `b4n`.
pub struct ResourcesView {
    pub table: ResourcesTable,
    app_data: SharedAppData,
    stats: SharedStatistics,
    generation: u16,
    modal: Dialog,
    command_palette: CommandPalette,
    filter: Filter,
}

impl ResourcesView {
    /// Creates a new resources view.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker) -> Self {
        let stats = worker.borrow().statistics.share();
        let generation = stats.borrow().generation;
        let table = ResourcesTable::new(Rc::clone(&app_data));
        let filter = Filter::new(Rc::clone(&app_data), Some(worker), 60);

        Self {
            table,
            app_data,
            stats,
            generation,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            filter,
        }
    }

    delegate! {
        to self.table {
            pub fn set_resources_info(&mut self, context: String, namespace: Namespace, version: String, scope: Scope);
            pub fn highlight_next(&mut self, resource_to_select: Option<String>);
            pub fn deselect_all(&mut self);
            pub fn kind_plural(&self) -> &str;
            pub fn scope(&self) -> &Scope;
            pub fn group(&self) -> &str;
            pub fn get_kind(&self) -> Kind;
            pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&ResourceItem>;
            pub fn set_namespace(&mut self, namespace: Namespace);
            pub fn set_view(&mut self, view: ViewType);
        }
    }

    /// Clears data in the list.
    pub fn clear_list_data(&mut self) {
        self.table.reset();
        self.filter.reset();
    }

    /// Updates resources list with a new data from [`ObserverResult`].
    pub fn update_resources_list(&mut self, result: ObserverResult<ResourceItem>) {
        if matches!(result, ObserverResult::Init(_)) {
            self.filter.reset();
            self.table.set_filter("");
        }

        self.table.update_resources_list(result);
    }

    /// Updates statistics if current resource kind is `pods` or `nodes`.
    pub fn update_statistics(&mut self) {
        let stats = &self.stats.borrow();
        if stats.generation == self.generation {
            return;
        }

        if self.table.kind_plural() == PODS {
            if let Some(items) = &mut self.table.list.table.table.list.items {
                pod::update_statistics(items.full_iter_mut(), stats);
            }
        } else if self.table.kind_plural() == NODES
            && let Some(items) = &mut self.table.list.table.table.list.items
        {
            node::update_statistics(items.full_iter_mut(), stats);
        }

        self.generation = stats.generation;
    }

    /// Shows delete resources dialog if anything is selected.
    pub fn ask_delete_resources(&mut self) {
        if self.table.list.table.is_anything_selected() && !self.table.has_containers() {
            self.modal = self.new_delete_dialog();
            self.modal.show();
        }
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: Vec<NamedContext>) {
        let actions_list = ActionsListBuilder::from_contexts(&list).build();
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 60)
            .with_prompt("context")
            .with_selected(&self.app_data.borrow().current.context);
        self.command_palette.show();
    }

    /// Displays a list of available themes to choose from.
    pub fn show_themes_list(&mut self, list: Vec<std::path::PathBuf>) {
        let actions_list = ActionsListBuilder::from_paths(list).build();
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 60)
            .with_prompt("theme")
            .with_selected(&self.app_data.borrow().config.theme);
        self.command_palette.show();
    }

    /// Displays a list of available forward ports for a container to choose from.
    pub fn show_ports_list(&mut self, list: Vec<Port>) {
        if let Some(resource) = self.table.get_resource_ref(true) {
            let actions_list = ActionsListBuilder::from_resource_ports(&list).build();
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 60)
                .with_header(format!(
                    " Add port forward for {} container:",
                    resource.container.as_deref().unwrap_or_default()
                ))
                .with_prompt("container port")
                .with_validator(ValidatorKind::Number(0, 65_535))
                .with_step(
                    StepBuilder::input("")
                        .with_validator(ValidatorKind::Number(0, 65_535))
                        .with_prompt("local port")
                        .build(),
                )
                .with_step(
                    StepBuilder::input("127.0.0.1")
                        .with_validator(ValidatorKind::IpAddr)
                        .with_prompt("bind address")
                        .build(),
                )
                .with_response(|v| build_port_forward_response(v, resource));
            self.command_palette.show();
        }
    }

    /// Processes disconnection state.
    pub fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    /// Returns `true` if namespaces selector can be displayed.
    pub fn is_namespaces_selector_allowed(&self) -> bool {
        self.table.scope() == &Scope::Namespaced
            && !self.table.has_containers()
            && !self.table.has_resources_events()
            && self.is_resources_selector_allowed()
    }

    /// Returns `true` if resources selector can be displayed.
    pub fn is_resources_selector_allowed(&self) -> bool {
        !self.filter.is_visible && !self.modal.is_visible && !self.command_palette.is_visible
    }

    /// Draws [`ResourcesView`] on the provided frame and area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.table.draw(frame, area);

        self.modal.draw(frame, frame.area());
        self.command_palette.draw(frame, frame.area());
        self.filter.draw(frame, frame.area());
    }

    /// Process TUI event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.modal.is_visible {
            return self.modal.process_event(event);
        }

        if self.command_palette.is_visible {
            return self.process_command_palette_event(event);
        }

        if !self.app_data.borrow().is_connected {
            if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen)
                || event.is_in(MouseEventKind::RightClick, self.table.list.area)
            {
                self.show_command_palette(true);
            }

            return ResponseEvent::Handled;
        }

        if self.filter.is_visible {
            let result = self.filter.process_event(event);
            self.table.set_filter(self.filter.value());
            return result;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateDelete) {
            self.ask_delete_resources();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::FilterReset) && !self.filter.value().is_empty() {
            self.filter.reset();
            self.table.set_filter("");
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::FilterOpen) {
            self.filter.show();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen) {
            self.show_command_palette(false);
        }

        if event.is_in(MouseEventKind::RightClick, self.table.list.area) {
            self.show_command_palette(true);
        }

        self.table.process_event(event)
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.command_palette
            .process_event(event)
            .when_action_then("back", || {
                self.process_event(&self.app_data.get_event(KeyCommand::NavigateBack))
            })
            .when_action_then("filter", || {
                self.process_event(&self.app_data.get_event(KeyCommand::FilterOpen))
            })
            .when_action_then("show_events", || {
                self.table.process_event(&self.app_data.get_event(KeyCommand::EventsShow))
            })
            .when_action_then("show_yaml", || {
                self.table.process_event(&self.app_data.get_event(KeyCommand::YamlOpen))
            })
            .when_action_then("decode_yaml", || {
                self.table.process_event(&self.app_data.get_event(KeyCommand::YamlDecode))
            })
            .when_action_then("show_logs", || {
                self.table.process_event(&self.app_data.get_event(KeyCommand::LogsOpen))
            })
            .when_action_then("show_plogs", || {
                self.table
                    .process_event(&self.app_data.get_event(KeyCommand::PreviousLogsOpen))
            })
            .when_action_then("open_shell", || {
                self.table.process_event(&self.app_data.get_event(KeyCommand::ShellOpen))
            })
            .when_action_then("port_forward", || {
                self.table
                    .process_event(&self.app_data.get_event(KeyCommand::PortForwardsCreate))
            })
    }

    fn show_command_palette(&mut self, simplifed: bool) {
        if !self.app_data.borrow().is_connected {
            let actions = ActionsListBuilder::default().with_resources_actions(false).build();
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions, 60);
            self.command_palette.show();
            return;
        }

        let is_containers = self.table.kind_plural() == CONTAINERS;
        let is_pods = self.table.kind_plural() == PODS;
        let mut builder = ActionsListBuilder::from_kinds(self.app_data.borrow().kinds.as_deref(), simplifed)
            .with_resources_actions(!is_containers)
            .with_forwards()
            .with_action(
                ActionItem::new("show YAML")
                    .with_description(if is_containers {
                        "shows YAML of the container's resource"
                    } else {
                        "shows YAML of the selected resource"
                    })
                    .with_aliases(&["yaml", "yml"])
                    .with_response(ResponseEvent::Action("show_yaml")),
            )
            .with_action(
                ActionItem::new("back")
                    .with_description("returns to the previous view")
                    .with_response(ResponseEvent::Action("back")),
            )
            .with_action(
                ActionItem::new("filter")
                    .with_description("shows resources filter input")
                    .with_response(ResponseEvent::Action("filter")),
            );

        if !is_containers {
            builder = builder.with_action(
                ActionItem::new("show events")
                    .with_description("shows events for the selected resource")
                    .with_response(ResponseEvent::Action("show_events")),
            );
        }

        if is_containers || is_pods {
            builder = builder
                .with_action(
                    ActionItem::new("show logs")
                        .with_description("shows container logs")
                        .with_aliases(&["logs"])
                        .with_response(ResponseEvent::Action("show_logs")),
                )
                .with_action(
                    ActionItem::new("show previous logs")
                        .with_description("shows container previous logs")
                        .with_aliases(&["previous"])
                        .with_response(ResponseEvent::Action("show_plogs")),
                )
                .with_action(
                    ActionItem::new("shell")
                        .with_description("opens container shell")
                        .with_response(ResponseEvent::Action("open_shell")),
                )
                .with_action(
                    ActionItem::new("forward port")
                        .with_description("forwards container port")
                        .with_aliases(&["port", "pf"])
                        .with_response(ResponseEvent::Action("port_forward")),
                );
        }

        if self.table.kind_plural() == SECRETS {
            builder = builder.with_action(
                ActionItem::new("decode")
                    .with_description("shows decoded YAML of the selected secret")
                    .with_aliases(&["decode", "x"])
                    .with_response(ResponseEvent::Action("decode_yaml")),
            );
        }

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
        self.command_palette.show();
    }

    /// Creates new delete dialog.
    fn new_delete_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "Are you sure you want to delete the selected resources?".to_owned(),
            vec![
                Button::new(
                    "Delete".to_owned(),
                    ResponseEvent::DeleteResources,
                    colors.modal.btn_delete.clone(),
                ),
                Button::new("Cancel".to_owned(), ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
            ],
            60,
            colors.modal.text,
        )
    }
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
