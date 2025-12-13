use b4n_common::NotificationSink;
use b4n_config::keys::KeyCommand;
use b4n_kube::stats::SharedStatistics;
use b4n_kube::{CONTAINERS, EVENTS, Kind, NAMESPACES, NODES, Namespace, ObserverResult, PODS, Port, ResourceRef, SECRETS};
use b4n_tui::widgets::{Button, CheckBox, Dialog, ValidatorKind};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, ScopeData, TuiEvent, table::Table, table::ViewType};
use delegate::delegate;
use kube::{config::NamedContext, discovery::Scope};
use ratatui::layout::Position;
use ratatui::{Frame, layout::Rect};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

use crate::core::{PreviousData, SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::kube::resources::{ResourceItem, node, pod};
use crate::ui::views::resources::{NextRefreshActions, table::ResourcesTable};
use crate::ui::widgets::{ActionItem, ActionsListBuilder, CommandPalette, Filter, StepBuilder};

/// Resources view (main view) for `b4n`.
pub struct ResourcesView {
    pub table: ResourcesTable,
    app_data: SharedAppData,
    stats: SharedStatistics,
    generation: u16,
    modal: Dialog,
    command_palette: CommandPalette,
    filter: Filter,
    footer_tx: NotificationSink,
}

impl ResourcesView {
    /// Creates a new resources view.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: NotificationSink) -> Self {
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
            footer_tx,
        }
    }

    delegate! {
        to self.table {
            pub fn set_resources_info(&mut self, context: String, namespace: Namespace, version: String, scope: Scope);
            pub fn set_next_refresh(&mut self, actions: NextRefreshActions);
            pub fn set_next_highlight(&mut self, actions: Option<String>);
            pub fn clear_header_scope(&mut self, clear_on_next: bool);
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
        let is_init = matches!(result, ObserverResult::Init(_));

        if is_init {
            // apply_filter must be checked before updating the table list, it is cleared there
            if let Some(filter) = self.table.next_refresh().apply_filter.as_deref() {
                self.filter.set_value(filter.to_owned());
            } else {
                self.filter.reset();
            }
        }

        self.table.update_resources_list(result);

        if is_init {
            // the breadcrumb trail must be updated after updating the table list
            self.update_breadcrumb_trail();
        }
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

    /// Updates error state for the resources table.
    pub fn update_error_state(&mut self, has_error: bool) {
        self.table.list.update_error_state(has_error);
    }

    /// Shows delete resources dialog if anything is selected.
    pub fn ask_delete_resources(&mut self) {
        if self.table.list.table.is_anything_selected() && !self.table.has_containers() && self.table.list.table.data.is_deletable
        {
            self.modal = self.new_delete_dialog();
            self.modal.show();
        }
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: &[NamedContext]) {
        let actions_list = ActionsListBuilder::from_contexts(list).build();
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 60)
            .with_prompt("context")
            .with_selected(&self.app_data.borrow().current.context);
        self.command_palette.show();
    }

    /// Displays a list of available themes to choose from.
    pub fn show_themes_list(&mut self, list: Vec<PathBuf>) {
        let actions_list = ActionsListBuilder::from_paths(list).build();
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 60)
            .with_prompt("theme")
            .with_selected(&self.app_data.borrow().config.theme);
        self.command_palette.show();
    }

    /// Displays a list of available forward ports for a container to choose from.
    pub fn show_ports_list(&mut self, list: &[Port]) {
        if let Some(resource) = self.table.get_resource_ref(true) {
            let actions_list = ActionsListBuilder::from_resource_ports(list).build();
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
                        .with_colors(self.app_data.borrow().theme.colors.command_palette.clone())
                        .build(),
                )
                .with_step(
                    StepBuilder::input("127.0.0.1")
                        .with_validator(ValidatorKind::IpAddr)
                        .with_prompt("bind address")
                        .with_colors(self.app_data.borrow().theme.colors.command_palette.clone())
                        .build(),
                )
                .with_response(|v| build_port_forward_response(v, resource));
            self.command_palette.show();
        }
    }

    /// Processes disconnection state.
    pub fn process_disconnection(&mut self) {
        self.command_palette.hide();
        self.update_error_state(true);
    }

    /// Returns `true` if namespaces selector can be displayed.
    pub fn is_namespaces_selector_allowed(&self) -> bool {
        self.table.scope() == &Scope::Namespaced
            && !self.table.has_containers()
            && !self.table.list.table.is_scoped()
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
            if self.modal.process_event(event).is_action("delete") {
                return ResponseEvent::DeleteResources(
                    self.modal.input(0).is_some_and(|i| i.is_checked), // terminate immediately
                    self.modal.input(1).is_some_and(|i| i.is_checked), // detach finalizers
                );
            }

            return ResponseEvent::Handled;
        }

        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled {
                return result;
            }
        }

        if !self.app_data.borrow().is_connected {
            if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen)
                || event.is_in(MouseEventKind::RightClick, self.table.list.area)
            {
                self.show_command_palette();
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
            self.show_command_palette();
            return ResponseEvent::Handled;
        }

        if let TuiEvent::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::RightClick
            && self.table.list.area.contains(Position::new(mouse.column, mouse.row))
        {
            let line_no = mouse.row.saturating_sub(self.table.list.area.y);
            if !self.table.list.table.highlight_item_by_line(line_no) {
                self.table.list.table.unhighlight_item();
            }
            self.show_mouse_menu(mouse.column, mouse.row);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::YamlCreate) {
            self.show_create_resource_palette();
            return ResponseEvent::Handled;
        }

        let result = self.table.process_event(event);
        if result == ResponseEvent::ViewPreviousResource {
            return self.handle_previous_resource_change();
        }

        result
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);
        if let ResponseEvent::Action(action) = response {
            return match action {
                "back" => self.process_event(&self.app_data.get_event(KeyCommand::NavigateBack)),
                "filter" => self.process_event(&self.app_data.get_event(KeyCommand::FilterOpen)),
                "create" => self.process_event(&self.app_data.get_event(KeyCommand::YamlCreate)),
                "show_events" => self.table.process_event(&self.app_data.get_event(KeyCommand::EventsShow)),
                "show_involved" => self
                    .table
                    .process_event(&self.app_data.get_event(KeyCommand::InvolvedObjectShow)),
                "show_yaml" => self.table.process_event(&self.app_data.get_event(KeyCommand::YamlOpen)),
                "edit_yaml" => self.table.process_event(&self.app_data.get_event(KeyCommand::YamlEdit)),
                "decode_yaml" => self.table.process_event(&self.app_data.get_event(KeyCommand::YamlDecode)),
                "show_logs" => self.table.process_event(&self.app_data.get_event(KeyCommand::LogsOpen)),
                "show_plogs" => self
                    .table
                    .process_event(&self.app_data.get_event(KeyCommand::PreviousLogsOpen)),
                "open_shell" => self.table.process_event(&self.app_data.get_event(KeyCommand::ShellOpen)),
                "port_forward" => self
                    .table
                    .process_event(&self.app_data.get_event(KeyCommand::PortForwardsCreate)),
                "new_clone" => self.create_new_resource(true, false),
                "new_full" => self.create_new_resource(false, true),
                "new_minimal" => self.create_new_resource(false, false),
                _ => response,
            };
        }

        response
    }

    fn show_command_palette(&mut self) {
        if !self.app_data.borrow().is_connected {
            let actions = ActionsListBuilder::default().with_resources_actions(false).build();
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions, 60);
            self.command_palette.show();
            return;
        }

        let is_highlighted = self.table.list.table.is_anything_highlighted();
        let is_containers = self.table.kind_plural() == CONTAINERS;
        let is_pods = self.table.kind_plural() == PODS;
        let is_events = self.table.kind_plural() == EVENTS;
        let is_deletable = self.table.list.table.is_anything_selected() && self.table.list.table.data.is_deletable;

        let mut builder = ActionsListBuilder::from_kinds(self.app_data.borrow().kinds.as_deref())
            .with_resources_actions(!is_containers && is_deletable)
            .with_forwards()
            .with_action(ActionItem::action("filter", "filter").with_description("shows resources filter input"));

        if self.table.kind_plural() != NAMESPACES {
            builder.add_action(ActionItem::action("back", "back").with_description("returns to the previous view"));
        }

        if !is_containers && !is_events {
            if is_highlighted {
                builder.add_action(
                    ActionItem::action("show events", "show_events").with_description("shows events for the selected resource"),
                );
            }

            if self.table.list.table.data.is_creatable {
                builder.add_action(
                    ActionItem::action("create", "create")
                        .with_description("creates new Kubernetes resource")
                        .with_aliases(&["new", "add"]),
                );
            }
        }

        if self.has_involved_object() {
            builder.add_action(
                ActionItem::action("involved object", "show_involved").with_description("navigates to the involved object"),
            );
        }

        if is_highlighted {
            builder.add_action(
                ActionItem::action("show YAML", "show_yaml")
                    .with_description(if is_containers {
                        "shows YAML of the container's resource"
                    } else {
                        "shows YAML of the highlighted resource"
                    })
                    .with_aliases(&["yaml", "yml", "view"]),
            );

            if is_containers || is_pods {
                builder = builder
                    .with_action(
                        ActionItem::action("show logs", "show_logs")
                            .with_description("shows container logs")
                            .with_aliases(&["logs"]),
                    )
                    .with_action(
                        ActionItem::action("show previous logs", "show_plogs")
                            .with_description("shows container previous logs")
                            .with_aliases(&["previous"]),
                    )
                    .with_action(ActionItem::action("shell", "open_shell").with_description("opens container shell"))
                    .with_action(
                        ActionItem::action("forward port", "port_forward")
                            .with_description("forwards container port")
                            .with_aliases(&["port", "pf"]),
                    );
            }

            if self.table.kind_plural() == SECRETS {
                builder.add_action(
                    ActionItem::action("decode", "decode_yaml")
                        .with_description("shows decoded YAML of the highlighted secret")
                        .with_aliases(&["decode", "x"]),
                );
            }

            if self.table.list.table.data.is_editable && self.table.kind_plural() != EVENTS {
                builder.add_action(
                    ActionItem::action("edit YAML", "edit_yaml")
                        .with_description("displays YAML and switches to edit mode")
                        .with_aliases(&["yaml", "yml", "patch"]),
                );
            }
        }

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
        self.command_palette.show();
    }

    fn show_mouse_menu(&mut self, x: u16, y: u16) {
        if !self.app_data.borrow().is_connected {
            return;
        }

        let is_highlighted = self.table.list.table.is_anything_highlighted();
        let is_containers = self.table.kind_plural() == CONTAINERS;
        let is_pods = self.table.kind_plural() == PODS;
        let is_events = self.table.kind_plural() == EVENTS;
        let mut builder = ActionsListBuilder::default();

        if self.table.kind_plural() != NAMESPACES {
            builder.add_action(ActionItem::menu("󰕍 back", "back"));
        }

        if self.table.list.table.is_anything_selected() && self.table.list.table.data.is_deletable {
            builder.add_action(ActionItem::menu(" delete", "").with_response(ResponseEvent::AskDeleteResources));
        }

        if !is_containers && !is_events {
            if is_highlighted {
                builder.add_action(ActionItem::menu("󰑏 events", "show_events"));
            }
            if self.table.list.table.data.is_creatable {
                builder.add_action(ActionItem::menu("󰐕 create new", "create"));
            }
        }

        if self.has_involved_object() {
            builder.add_action(ActionItem::menu("󰑏 involved object", "show_involved"));
        }

        if is_highlighted {
            builder.add_action(ActionItem::menu(" YAML", "show_yaml"));
            if is_containers || is_pods {
                builder = builder
                    .with_action(ActionItem::menu(" logs", "show_logs"))
                    .with_action(ActionItem::menu(" logs [previous]", "show_plogs"))
                    .with_action(ActionItem::menu(" shell", "open_shell"))
                    .with_action(ActionItem::menu(" forward port", "port_forward"));
            }

            if self.table.kind_plural() == SECRETS {
                builder.add_action(ActionItem::menu(" YAML [decoded]", "decode_yaml"));
            }

            if self.table.list.table.data.is_editable && self.table.kind_plural() != EVENTS {
                builder.add_action(ActionItem::menu(" edit", "edit_yaml"));
            }
        }

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 22).with_input(false);
        self.command_palette.show_at(x.saturating_sub(1), y);
    }

    fn show_create_resource_palette(&mut self) {
        if self.kind_plural() == CONTAINERS
            || self.kind_plural() == EVENTS
            || !self.table.list.table.data.is_creatable
            || !self.app_data.borrow().is_connected
        {
            return;
        }

        let mut builder = ActionsListBuilder::new()
            .with_action(ActionItem::action("full", "new_full").with_description("get all possible fields for the spec"))
            .with_action(ActionItem::action("minimal", "new_minimal").with_description("get only required fields for the spec"));

        if self.table.list.table.is_anything_highlighted() && self.kind_plural() != NAMESPACES {
            builder = builder.with_action(
                ActionItem::action("duplicate", "new_clone")
                    .with_description("use the spec of the highlighted resource")
                    .with_aliases(&["clone"]),
            );
        }

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60)
            .with_prompt("create new resource")
            .with_first_selected();
        self.command_palette.show();
    }

    fn new_delete_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "Are you sure you want to delete the selected resources?".to_owned(),
            vec![
                Button::new("Delete", ResponseEvent::Action("delete"), &colors.modal.btn_delete),
                Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
            ],
            60,
            colors.modal.text,
        )
        .with_inputs(vec![
            CheckBox::new(0, "Terminate immediately", false, &colors.modal.checkbox),
            CheckBox::new(1, "Remove finalizers before deletion", false, &colors.modal.checkbox),
        ])
    }

    pub fn remember_current_resource(&mut self) {
        let highlighted = self.table.list.table.get_highlighted_item_name().map(String::from);
        let header = self.table.header.get_scope();
        let namespace = self.app_data.borrow().current.namespace.clone();
        let resource = self.app_data.borrow().current.resource.clone();
        self.app_data.borrow_mut().previous.push(PreviousData {
            list: self.scope().clone(),
            header,
            highlighted,
            namespace,
            resource,
            filter: self.table.list.table.get_filter().map(String::from),
            sort_info: self.table.list.table.table.header.sort_info(),
            offset: self.table.list.table.offset(),
        });
    }

    fn handle_previous_resource_change(&mut self) -> ResponseEvent {
        let data = &mut self.app_data.borrow_mut();
        if let Some(previous) = data.previous.pop() {
            self.table.set_next_refresh(NextRefreshActions::from_previous(&previous));
            let to_select = previous.highlighted().map(String::from);
            if previous.resource.filter.is_some() {
                let scope = ScopeData {
                    list: previous.list,
                    header: previous.header,
                    filter: previous.resource.filter.unwrap(),
                };
                return ResponseEvent::ViewScopedPrev(previous.resource.kind.into(), previous.namespace.into(), to_select, scope);
            }

            return ResponseEvent::ChangeAndSelectPrev(previous.resource.kind.into(), previous.namespace.into(), to_select);
        }

        ResponseEvent::Handled
    }

    fn update_breadcrumb_trail(&self) {
        let data = self.app_data.borrow();
        let mut elements = data.previous.iter().map(PreviousData::get_kind_name).collect::<Vec<_>>();
        if !elements.is_empty() {
            if data.current.resource.is_container() {
                elements.push(CONTAINERS.to_owned());
            } else {
                elements.push(data.current.resource.kind.name().to_owned());
            }
        }

        self.footer_tx.set_breadcrumb_trail(elements);
    }

    fn create_new_resource(&self, is_clone: bool, is_full: bool) -> ResponseEvent {
        let resource = &self.app_data.borrow().current;
        if is_clone && let Some(current) = self.table.list.table.get_highlighted_resource() {
            ResponseEvent::NewYaml(
                ResourceRef::named(
                    resource.resource.kind.clone(),
                    current.namespace.as_deref().into(),
                    current.name.clone(),
                ),
                false,
            )
        } else {
            ResponseEvent::NewYaml(
                ResourceRef::new(resource.resource.kind.clone(), resource.namespace.clone()),
                is_full,
            )
        }
    }

    fn has_involved_object(&self) -> bool {
        self.table
            .list
            .table
            .get_highlighted_resource()
            .is_some_and(|res| res.involved_object.is_some())
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
