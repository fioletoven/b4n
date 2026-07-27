use b4n_common::NotificationSink;
use b4n_config::keys::KeyCommand;
use b4n_kube::plugins::PluginContext;
use b4n_kube::{CONTAINERS, EVENTS, Kind, NODES, Namespace, ObserverResult, PODS, Port, ResourceRef};
use b4n_list::Row;
use b4n_tui::table::{Table, ViewType};
use b4n_tui::widgets::{ActionsList, ActionsListBuilder, Dialog};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, ScopeData, ToSelectData, TuiEvent};
use delegate::delegate;
use kube::{config::NamedContext, discovery::Scope};
use ratatui::layout::Position;
use ratatui::{Frame, layout::Rect};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

use crate::core::{PreviousData, ResourcesInfo, SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::kube::extensions::ActionsListBuilderExt;
use crate::kube::resources::{ResourceItem, ResourcesList, node, pod};
use crate::ui::views::View;
use crate::ui::views::resources::{NextRefreshActions, table::ResourcesTable};
use crate::ui::views::resources::{dialogs, menus};
use crate::ui::widgets::{CommandPalette, FileSelector, Filter, NamespaceSelector};

/// Resources view (main view) for `b4n`.
pub struct ResourcesView {
    pub table: ResourcesTable,
    app_data: SharedAppData,
    worker: SharedBgWorker,
    last_stats_generation: u16,
    last_ports_generation: u16,
    last_mouse_click: Option<Position>,
    modal: Dialog,
    command_palette: CommandPalette,
    filter: Filter,
    namespace_picker: NamespaceSelector,
    file_picker: FileSelector,
    footer_tx: NotificationSink,
}

impl ResourcesView {
    /// Creates a new resources view.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: NotificationSink) -> Self {
        let last_stats_generation = worker.borrow().statistics_generation();
        let last_ports_generation = worker.borrow().port_forwards_list_generation();
        let table = ResourcesTable::new(Rc::clone(&app_data));
        let filter = Filter::new(Rc::clone(&app_data), Some(Rc::clone(&worker)), 65);
        let namespace_picker = NamespaceSelector::new(Rc::clone(&app_data), Some(Rc::clone(&worker)), 65);
        let file_picker = FileSelector::new(Rc::clone(&app_data), Rc::clone(&worker), 65, PathBuf::from("."));

        Self {
            table,
            app_data,
            worker,
            last_stats_generation,
            last_ports_generation,
            last_mouse_click: None,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            filter,
            namespace_picker,
            file_picker,
            footer_tx,
        }
    }

    delegate! {
        to self.table {
            pub fn set_resources_info(&mut self, context: String, namespace: Namespace, version: String, scope: Scope);
            pub fn set_next_refresh(&mut self, actions: NextRefreshActions);
            pub fn set_next_highlight(&mut self, to_select: ToSelectData);
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

    /// Resets the list.
    pub fn reset(&mut self) {
        self.table.list.table = ResourcesList::default().with_filter_settings(Some("e"));
        self.table.header.set_count(0);
        self.table.header.show_filtered_icon(false);
        self.filter.reset();
        self.namespace_picker.reset();
    }

    /// Caches and clears data in the list.
    pub fn cache_list_data(&mut self) {
        self.table.move_to_cache();
        self.filter.reset();
        self.namespace_picker.reset();
    }

    /// Restores data in the list from cache.
    pub fn restore_list_data(&mut self, key: &str) {
        if self.table.restore_from_cache(key) {
            self.update_breadcrumb_trail();
            self.update_port_forwards();
        }
    }

    /// Updates resources list with a new data from [`ObserverResult`].
    pub fn update_resources_list(&mut self, result: ObserverResult<ResourceItem>) {
        let is_init = matches!(result, ObserverResult::Init(_));
        let is_init_done = matches!(result, ObserverResult::InitDone);

        if is_init {
            if self.app_data.borrow().is_pinned {
                if let Some(filter) = &self.app_data.borrow().pinned_filter {
                    self.filter.set_value(filter.to_owned());
                } else {
                    self.filter.reset();
                }
            } else if let Some(filter) = self.table.next_refresh().apply_filter.as_deref() {
                // apply_filter must be checked before updating the table list, it is cleared there
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

        if !is_init && !is_init_done {
            self.update_port_forwards();
        }
    }

    /// Updates statistics if current resource kind is `pods` or `nodes`.
    pub fn update_statistics(&mut self) {
        let worker = &self.worker.borrow();
        let stats = worker.statistics.stats().borrow();
        if stats.generation == self.last_stats_generation {
            return;
        }

        if self.table.kind_plural() == PODS {
            pod::update_statistics(self.table.list.table.table.list.full_iter_mut(), &stats);
            self.table.list.table.resort();
        } else if self.table.kind_plural() == NODES {
            node::update_statistics(self.table.list.table.table.list.full_iter_mut(), &stats);
            self.table.list.table.resort();
        }

        self.last_stats_generation = stats.generation;
    }

    /// Updates API error state for the resources table.
    pub fn update_error_state(&mut self, has_api_error: bool) {
        self.table.header.update_error_state(has_api_error);
        self.table.list.update_error_state(has_api_error);
    }

    /// Updates all elements that could change in external view.
    pub fn process_external_view_close(&mut self) {
        if self.app_data.borrow().is_pinned
            && let Some(filter) = self.app_data.borrow().pinned_filter.clone()
        {
            self.filter.set_value(filter);
            self.table.set_filter(self.filter.value());
        }
    }

    /// Shows delete resources dialog if anything is selected.
    pub fn ask_delete_resources(&mut self) {
        if self.table.list.table.is_anything_selected() && !self.table.has_containers() && self.table.list.table.data.is_deletable
        {
            self.modal = dialogs::new_delete_dialog(&self.app_data, self.last_mouse_click.take());
            self.modal.show();
        }
    }

    /// Shows stop port forwarding rules dialog if anything is selected.
    pub fn ask_stop_port_forwards(&mut self) {
        if let Some(resource) = self.table.list.table.get_highlighted_item_name().map(String::from) {
            self.modal = dialogs::new_stop_port_forwards_dialog(&self.app_data, self.last_mouse_click.take(), &resource);
            self.modal.show();
        }
    }

    /// Shows confirmation dialog for ephemeral container injection.
    pub fn ask_inject_container(&mut self, response: ResponseEvent) {
        if let ResponseEvent::InjectContainer(resource, container) = response {
            self.modal = dialogs::new_inject_container_dialog(&self.app_data, self.last_mouse_click.take(), resource, container);
            self.modal.show();
        }
    }

    /// Shows transfer file dialog.
    pub fn ask_transfer_file(&mut self, is_download: bool) {
        if let Some(resource) = self.table.list.table.get_highlighted_resource() {
            let tags = resource.data.as_ref().map(|d| d.tags.as_ref()).unwrap_or_default();
            self.modal = dialogs::new_transfer_dialog(is_download, &self.app_data, tags, self.last_mouse_click.take());
            self.modal.show();
        }
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: &[NamedContext]) {
        let actions_list = ActionsListBuilder::from_kube_contexts(list).build(None);
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 65)
            .with_prompt("context")
            .with_highlighted(&self.app_data.borrow().current.context);
        self.command_palette.show();
    }

    /// Displays a list of available themes to choose from.
    pub fn show_themes_list(&mut self, list: Vec<PathBuf>) {
        let actions_list = ActionsListBuilder::from_paths(list).build(None);
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 65)
            .with_prompt("theme")
            .with_highlighted(&self.app_data.borrow().config.theme);
        self.command_palette.show();
    }

    /// Displays a list of known namespaces to choose from.
    pub fn show_namespaces_list(&mut self, discovered: Vec<String>) {
        self.namespace_picker.set_discovered(discovered);
        self.namespace_picker.show();
        self.namespace_picker
            .highlight_item(self.app_data.borrow().current.namespace.as_str());
    }

    /// Displays a list of available forward ports for a container to choose from.
    pub fn show_ports_list(&mut self, list: &[Port]) {
        if let Some(resource) = self.table.get_resource_ref(true) {
            self.command_palette = menus::build_port_forward_steps(&self.app_data, resource, list)
                .with_highlighted_position(self.last_mouse_click.take());
            self.command_palette.show();
        }
    }

    fn process_widget_event(&mut self, event: &TuiEvent) -> Option<ResponseEvent> {
        if self.file_picker.is_visible {
            if self.file_picker.process_event(event) == ResponseEvent::Accepted {
                dialogs::update_transfer_dialog_paths(&mut self.modal, &self.file_picker);
            }

            return Some(ResponseEvent::Handled);
        }

        if self.modal.is_visible {
            let response = self.modal.process_event(event);

            if response.is_action("delete") {
                return Some(ResponseEvent::DeleteResources(
                    self.modal.selector(0).map(|s| s.selected().into()).unwrap_or_default(), // policy
                    self.modal.checkbox(0).is_some_and(|i| i.is_checked),                    // terminate immediately
                    self.modal.checkbox(1).is_some_and(|i| i.is_checked),                    // detach finalizers
                ));
            }

            if response.is_action("stop_port_forwards") {
                return Some(self.stop_port_forwards());
            }

            if response.is_action("select_file") {
                self.show_file_picker();
                return Some(ResponseEvent::Handled);
            }

            if response.is_action("transfer_file") {
                if let Some(resource) = self.table.get_resource_ref(false)
                    && let Some(context) = dialogs::get_transfer_dialog_context(&self.modal)
                {
                    return Some(ResponseEvent::TrnsferFile(resource, context));
                }
            } else {
                return Some(ResponseEvent::Handled);
            }

            if let ResponseEvent::PluginAction(plugin) = response {
                let info = &self.app_data.borrow().current;
                return Some(ResponseEvent::RunPlugin(
                    plugin.id,
                    build_plugin_context(info, &self.table, plugin.highlighted, plugin.selected),
                ));
            }

            if matches!(response, ResponseEvent::InjectContainer(_, _)) {
                return Some(response);
            }

            return Some(ResponseEvent::Handled);
        }

        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if matches!(result, ResponseEvent::InjectContainer(_, _)) {
                self.ask_inject_container(result);
                return Some(ResponseEvent::Handled);
            }
            if result != ResponseEvent::NotHandled {
                return Some(result);
            }
        }

        if self.filter.is_visible {
            let result = self.filter.process_event(event);
            if self.filter.is_valid() {
                self.table.set_filter(self.filter.value());
                self.filter.update_pinned_filter();
            }

            return Some(result);
        }

        if self.namespace_picker.is_visible {
            return Some(self.namespace_picker.process_event(event));
        }

        None
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);
        if response == ResponseEvent::AskDeleteResources {
            self.last_mouse_click = event.position();
        } else if let ResponseEvent::PluginAction(plugin) = response {
            if plugin.confirm {
                self.modal = dialogs::new_run_plugin_dialog(&self.app_data, self.last_mouse_click.take(), plugin);
                self.modal.show();
                return ResponseEvent::Handled;
            }
            let info = &self.app_data.borrow().current;
            return ResponseEvent::RunPlugin(
                plugin.id,
                build_plugin_context(info, &self.table, plugin.highlighted, plugin.selected),
            );
        } else if let ResponseEvent::Action(action) = response {
            return match action {
                "back" => self.process_event(&TuiEvent::Command(KeyCommand::NavigateBack)),
                "copy" => self.process_event(&TuiEvent::Command(KeyCommand::ContentCopy)),
                "copy_name" => {
                    self.copy_name_to_clipboard();
                    ResponseEvent::Handled
                },
                "palette" => {
                    self.last_mouse_click = event.position();
                    self.process_event(&TuiEvent::Command(KeyCommand::CommandPaletteOpen))
                },
                "filter" => {
                    self.last_mouse_click = event.position();
                    self.process_event(&TuiEvent::Command(KeyCommand::FilterOpen))
                },
                "pin_filter" => self.process_event(&TuiEvent::Command(KeyCommand::FilterPin)),
                "create" => {
                    self.last_mouse_click = event.position();
                    self.process_event(&TuiEvent::Command(KeyCommand::YamlCreate))
                },
                "show_events" => self.table.process_event(&TuiEvent::Command(KeyCommand::EventsShow)),
                "show_involved" => self.table.process_event(&TuiEvent::Command(KeyCommand::InvolvedObjectShow)),
                "show_yaml" => self.table.process_event(&TuiEvent::Command(KeyCommand::YamlOpen)),
                "edit_yaml" => self.table.process_event(&TuiEvent::Command(KeyCommand::YamlEdit)),
                "decode_yaml" => self.table.process_event(&TuiEvent::Command(KeyCommand::YamlDecode)),
                "show_logs" => self.table.process_event(&TuiEvent::Command(KeyCommand::LogsOpen)),
                "show_plogs" => self.table.process_event(&TuiEvent::Command(KeyCommand::PreviousLogsOpen)),
                "describe" => self.table.process_event(&TuiEvent::Command(KeyCommand::DescribeOpen)),
                "inject" => {
                    self.last_mouse_click = event.position();
                    self.process_event(&TuiEvent::Command(KeyCommand::ContainerInject))
                },
                "attach" => self.table.process_event(&TuiEvent::Command(KeyCommand::ContainerAttach)),
                "open_shell" => self.table.process_event(&TuiEvent::Command(KeyCommand::ShellOpen)),
                "port_forward" => {
                    self.last_mouse_click = event.position();
                    self.table.process_event(&TuiEvent::Command(KeyCommand::PortForwardsCreate))
                },
                "ask_stop_port_forwards" => {
                    self.last_mouse_click = event.position();
                    self.ask_stop_port_forwards();
                    ResponseEvent::Handled
                },
                "new_clone" => self.create_new_resource(true, false),
                "new_full" => self.create_new_resource(false, true),
                "new_minimal" => self.create_new_resource(false, false),
                _ => response,
            };
        }

        response
    }

    fn show_command_palette(&mut self) {
        if !self.app_data.borrow().is_connected() {
            let actions = ActionsListBuilder::default()
                .with_resources_actions(false)
                .build(Some(&self.app_data.borrow().key_bindings));

            self.open_command_palette(actions);
            return;
        }

        let actions = menus::build_resources_actions(&self.app_data, &self.table);
        self.open_command_palette(actions);
    }

    fn open_command_palette(&mut self, actions: ActionsList) {
        self.command_palette =
            CommandPalette::new(Rc::clone(&self.app_data), actions, 65).with_highlighted_position(self.last_mouse_click.take());
        self.command_palette.show();
        self.footer_tx.hide_hint();
    }

    fn show_mouse_menu(&mut self, x: u16, y: u16) {
        if !self.app_data.borrow().is_connected() {
            return;
        }

        let actions = menus::build_mouse_menu_actions(&self.table);
        let width = u16::try_from(actions.max_item_len() + 4).unwrap_or(u16::MAX).max(22);

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions, width).to_mouse_menu();
        self.command_palette.show_at((x.saturating_sub(3), y).into());
    }

    fn show_create_resource_palette(&mut self) {
        if self.kind_plural() == CONTAINERS
            || self.kind_plural() == EVENTS
            || !self.table.list.table.data.is_creatable
            || !self.app_data.borrow().is_connected()
        {
            return;
        }

        let actions = menus::build_create_resource_actions(&self.table);
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions, 65)
            .with_prompt("create new resource")
            .with_first_highlighted()
            .with_highlighted_position(self.last_mouse_click.take());
        self.command_palette.show();
    }

    fn show_ephemeral_containers_palette(&mut self) {
        if let Some(resource) = self.table.get_resource_ref(true) {
            let tags = self.table.get_resource_tags();
            self.command_palette = menus::build_ephemeral_container_steps(&self.app_data, resource, tags);
            self.command_palette.show();
        }
    }

    fn show_file_picker(&mut self) {
        let is_download = self.modal.checkbox(0).is_some_and(|cb| cb.is_checked);
        self.file_picker.set_dir_picker(is_download);
        self.file_picker
            .set_current_path(std::env::current_dir().unwrap_or(PathBuf::from(".")));
        self.file_picker.reset();
        self.file_picker.show();
    }

    pub fn remember_current_resource(&mut self) {
        let highlighted = self.table.list.table.get_highlighted_item_name_and_group();
        let highlighted = highlighted.map_or(ToSelectData::None, |(i, g)| ToSelectData::Some(i.to_owned(), g.to_owned()));
        let header = self.table.header.get_scope();
        let namespace = self.app_data.borrow().current.namespace.clone();
        let resource = self.app_data.borrow().current.resource.clone();
        self.app_data.borrow_mut().previous.push(PreviousData {
            list: self.scope().clone(),
            header,
            highlighted,
            namespace,
            resource,
            filter: self.table.list.table.filter().map(String::from),
            sort_info: self.table.list.table.table.header.sort_info(),
            offset: self.table.list.table.offset(),
        });
    }

    fn handle_previous_resource_change(&mut self) -> ResponseEvent {
        let data = &mut self.app_data.borrow_mut();
        if let Some(previous) = data.previous.pop() {
            self.table.set_next_refresh(NextRefreshActions::from_previous(&previous));
            let to_select = previous.highlighted;
            if let Some(filter) = previous.resource.filter {
                let scope = ScopeData {
                    list: previous.list,
                    header: previous.header,
                    filter,
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

    fn copy_name_to_clipboard(&mut self) {
        if let Some(res) = self.table.list.table.get_highlighted_resource() {
            self.app_data
                .copy_to_clipboard(&res.name, &self.footer_tx, || "Resource name copied to clipboard");
        }
    }

    fn get_mouse_menu_position(&self, line_no: u16, resource_name: &str) -> Position {
        self.table
            .list
            .table
            .table
            .get_mouse_menu_position(line_no, resource_name, self.table.list.area)
    }

    fn update_port_forwards(&mut self) {
        if self.table.kind_plural() == PODS {
            let namespace = &self.table.list.table.data.resource.namespace;
            let worker = &mut self.worker.borrow_mut();
            let new_list = worker.get_port_forward_refs(namespace);
            self.table.list.table.update_port_forwards(&new_list);
        }
    }

    fn stop_port_forwards(&self) -> ResponseEvent {
        if let Some(resource) = self.table.list.table.get_highlighted_resource() {
            let containers = resource.to_containers_vec();
            self.worker.borrow_mut().stop_container_port_forwards(&containers);
            self.footer_tx.show_info(
                format!("Port forwarding rules for '{}' have been stopped", resource.name()),
                3_000,
            );
        }

        ResponseEvent::Handled
    }
}

impl View for ResourcesView {
    fn is_namespaces_selector_allowed(&self) -> bool {
        self.table.scope() == &Scope::Namespaced
            && !self.table.has_containers()
            && !self.table.list.table.is_scoped()
            && self.is_resources_selector_allowed()
    }

    fn is_resources_selector_allowed(&self) -> bool {
        !self.filter.is_visible && !self.modal.is_visible && !self.command_palette.is_visible && !self.namespace_picker.is_visible
    }

    fn process_tick(&mut self) -> ResponseEvent {
        self.table.list.table.remove_expired_cache_entries();

        let generation = self.worker.borrow().port_forwards_list_generation();
        if self.last_ports_generation != generation {
            self.last_ports_generation = generation;
            self.update_port_forwards();
        }

        ResponseEvent::Handled
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let Some(result) = self.process_widget_event(event) {
            return result;
        }

        if !self.app_data.borrow().is_connected() {
            if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen)
                || event.is_in(MouseEventKind::RightClick, self.table.list.area)
            {
                self.show_command_palette();
                return ResponseEvent::Handled;
            }

            return ResponseEvent::NotHandled;
        }

        let is_highlighted = self.table.list.table.is_anything_highlighted();
        let is_selected = self.table.list.table.is_anything_selected();
        if let Some(plugin) = self
            .app_data
            .get_plugin_binding(event, self.table.get_kind().as_str(), is_highlighted, is_selected)
        {
            if plugin.confirm {
                self.modal = dialogs::new_run_plugin_dialog(&self.app_data, self.last_mouse_click.take(), plugin);
                self.modal.show();
                return ResponseEvent::Handled;
            }

            let info = &self.app_data.borrow().current;
            return ResponseEvent::RunPlugin(
                plugin.id,
                build_plugin_context(info, &self.table, plugin.highlighted, plugin.selected),
            );
        }

        if self.app_data.has_binding(event, KeyCommand::ContentCopy) {
            self.table
                .copy_to_clipboard(self.table.list.table.is_anything_selected(), &self.footer_tx);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateDelete) {
            self.ask_delete_resources();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::FilterPin) {
            return self.filter.toggle_pin();
        }

        if self.filter.is_reset_filter_event(event) {
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

        if self.app_data.has_binding(event, KeyCommand::MouseMenuOpen)
            && let Some(line_no) = self.table.list.table.get_highlighted_item_line_no()
            && let Some(item_name) = self.table.list.table.get_highlighted_item_name()
        {
            let pos = self.get_mouse_menu_position(line_no, item_name);
            self.show_mouse_menu(pos.x, pos.y);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::YamlCreate) {
            self.show_create_resource_palette();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::ContainerInject) {
            self.show_ephemeral_containers_palette();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::TransferTo) {
            self.ask_transfer_file(false);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::TransferFrom) {
            self.ask_transfer_file(true);
            return ResponseEvent::Handled;
        }

        let result = self.table.process_event(event);
        if result == ResponseEvent::ViewPreviousResource {
            return self.handle_previous_resource_change();
        }

        result
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, _has_focus: bool) {
        self.table.draw(frame, area);

        let area = frame.area();
        self.modal.draw(frame, area);
        self.command_palette.draw(frame, area);
        self.filter.draw(frame, area);
        self.namespace_picker.draw(frame, area);
        self.file_picker.draw(frame, area);
    }
}

fn build_plugin_context(info: &ResourcesInfo, table: &ResourcesTable, is_highlighted: bool, is_selected: bool) -> PluginContext {
    let mut resources = Vec::new();
    let mut values = Vec::new();

    if is_highlighted {
        if let Some(resource) = table.get_resource_ref(false) {
            resources.push(resource);
        }

        if let Some(result) = table.get_column_values() {
            values.push(result);
        }
    }

    if is_selected {
        resources.append(&mut table.get_selected_resources_ref(false));
        values.append(&mut table.get_selected_column_values());
    }

    PluginContext {
        context: info.context.clone(),
        kind: info.resource.kind.clone(),
        namespace: info.namespace.clone(),
        resources,
        columns: table.get_column_names(),
        values,
    }
}
