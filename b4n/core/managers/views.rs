use anyhow::Result;
use b4n_common::{DEFAULT_ERROR_DURATION, IconKind, NotificationSink};
use b4n_config::keys::KeyCommand;
use b4n_kube::{ALL_NAMESPACES, ContainerRef, Namespace, PODS, Port, PropagationPolicy, ResourceRef, ResourceTag};
use b4n_tasks::commands::{
    CommandResult, DeleteResourcesOptions, GetNewResourceYamlError, GetNewResourceYamlResult, ResourceYamlError,
    ResourceYamlResult, SetNewResourceYamlError, SetResourceYamlError,
};
use b4n_tui::widgets::Footer;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table, table::ViewType};
use kube::{config::NamedContext, discovery::Scope};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::collections::HashMap;
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::kube::resources::ResourceItem;
use crate::kube::{kinds::KindsList, resources::ResourcesList};
use crate::ui::views::{ForwardsView, LogsView, ResourcesView, ShellView, View, YamlView};
use crate::ui::widgets::{Position, SideSelect};

pub struct ViewsManager {
    app_data: SharedAppData,
    worker: SharedBgWorker,
    resources: ResourcesView,
    ns_selector: SideSelect<ResourcesList>,
    res_selector: SideSelect<KindsList>,
    view: Option<Box<dyn View>>,
    footer: Footer,
    workspace: Rect,
    areas: Vec<Rect>,
}

impl ViewsManager {
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, resources: ResourcesView, footer: Footer) -> Self {
        let ns_selector = SideSelect::new(Rc::clone(&app_data), ResourcesList::default(), Position::Left, 30)
            .with_name("NAMESPACE")
            .with_result(ResponseEvent::ChangeNamespace)
            .with_quick_highlight(ALL_NAMESPACES);
        let res_selector = SideSelect::new(Rc::clone(&app_data), KindsList::default(), Position::Right, 40)
            .with_name("RESOURCE")
            .with_result(ResponseEvent::ChangeKind)
            .with_quick_highlight(PODS);
        set_command_palette_hint(footer.transmitter(), &app_data);

        Self {
            app_data,
            worker,
            resources,
            ns_selector,
            res_selector,
            view: None,
            footer,
            workspace: Rect::default(),
            areas: vec![Rect::default(), Rect::default()],
        }
    }

    /// Returns footer transmitter.
    pub fn footer(&self) -> &NotificationSink {
        self.footer.transmitter()
    }

    /// Updates all lists with observed resources.
    pub fn update_lists(&mut self) {
        let mut worker = self.worker.borrow_mut();
        if !worker.resources.is_running() {
            return;
        }

        // If this is not a built-in Kubernetes API group, wait for the CRD list to become ready
        // before polling anything else. This ensures that the header for the current resource
        // (if it is a custom resource) is shown only after all columns are known.
        let is_crds_list_ready = worker.ensure_crds_list_is_ready();
        if !worker.resources.observed_kind().is_builtin() && !is_crds_list_ready {
            return;
        }

        worker.update_crds_list();
        worker.update_statistics();

        if worker.update_discovery_list() {
            self.res_selector.select.items.update(worker.get_kinds_list(), 1, false);
            self.app_data.borrow_mut().kinds = self.res_selector.select.items.to_vec();
        }

        while let Some(update_result) = worker.namespaces.try_next() {
            self.ns_selector.select.items.update(*update_result);
        }

        while let Some(update_result) = worker.resources.try_next() {
            self.resources.update_resources_list(*update_result);
        }

        self.resources.update_statistics();
        self.resources.update_error_state(worker.resources.has_api_error());
    }

    /// Draws visible views on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let layout = Footer::get_layout(frame.area());
        self.workspace = layout[0];
        self.footer.show_breadcrumb_trail(self.view.is_none());
        self.footer.draw(frame, layout[1], &self.app_data.borrow().theme);

        if let Some(view) = &mut self.view {
            view.draw(frame, layout[0]);
        } else {
            self.resources.draw(frame, layout[0]);
        }

        self.draw_selectors(frame, layout[0]);
        self.footer.draw_history(frame, layout[0], &self.app_data.borrow().theme);
    }

    /// Draws namespace / resource selector located on the left / right of the views.
    fn draw_selectors(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if self.ns_selector.is_visible || self.res_selector.is_visible {
            let bottom = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
                .split(area);

            self.ns_selector.draw(frame, bottom[1]);
            self.res_selector.draw(frame, bottom[1]);
        }

        self.areas[0] = Rect::new(area.x, area.y + 1, 4, area.height);
        self.areas[1] = Rect::new(area.x + area.width.saturating_sub(4), area.y + 1, 4, area.height);
    }

    /// Processes single TUI event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.footer.is_message_history_visible() {
            if self.app_data.has_binding(event, KeyCommand::HistoryOpen)
                || self.app_data.has_binding(event, KeyCommand::NavigateBack)
            {
                self.footer.hide_message_history();
                return ResponseEvent::Handled;
            }

            if self.app_data.has_binding(event, KeyCommand::ContentCopy) {
                self.copy_footer_message();
            }

            return self.footer.process_event(event);
        }

        if self.ns_selector.is_visible {
            let result = self.ns_selector.process_event(event);
            if let Some(view) = &mut self.view {
                view.handle_namespaces_selector_event(&result);
            }

            if result != ResponseEvent::NotHandled {
                return result;
            }
        }

        if self.res_selector.is_visible {
            let result = self.res_selector.process_event(event);
            if let Some(view) = &mut self.view {
                view.handle_resources_selector_event(&result);
            }

            if result != ResponseEvent::NotHandled {
                return result;
            }
        }

        let result = if self.view.is_some() {
            self.process_view_event(event)
        } else {
            self.process_resources_event(event)
        };

        if result == ResponseEvent::NotHandled {
            if self.app_data.has_binding(event, KeyCommand::HistoryOpen) {
                self.footer.show_message_history();
                return ResponseEvent::Handled;
            }

            return self.footer.process_event(event);
        }

        result
    }

    fn process_view_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let Some(view) = &mut self.view else {
            return ResponseEvent::NotHandled;
        };

        if self.app_data.borrow().is_connected() {
            if (self.app_data.has_binding(event, KeyCommand::SelectorLeft)
                || event.is_in(MouseEventKind::RightClick, self.areas[0]))
                && self.worker.borrow().namespaces.has_access()
                && view.is_namespaces_selector_allowed()
            {
                self.ns_selector.show_selected(view.displayed_namespace());
                return ResponseEvent::Handled;
            }

            if (self.app_data.has_binding(event, KeyCommand::SelectorRight)
                || event.is_in(MouseEventKind::RightClick, self.areas[1]))
                && view.is_resources_selector_allowed()
            {
                self.res_selector
                    .show_selected_uid(self.resources.table.get_kind_for_selector().as_str());
                return ResponseEvent::Handled;
            }
        }

        let response = view.process_event(event);
        if response == ResponseEvent::Cancelled {
            self.view = None;
            return ResponseEvent::Handled;
        }

        response
    }

    fn process_resources_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.app_data.borrow().is_connected() {
            if (self.app_data.has_binding(event, KeyCommand::SelectorLeft)
                || event.is_in(MouseEventKind::RightClick, self.areas[0]))
                && self.worker.borrow().namespaces.has_access()
                && self.resources.is_namespaces_selector_allowed()
            {
                self.ns_selector
                    .show_selected(self.app_data.borrow().current.namespace.as_str());
                return ResponseEvent::Handled;
            }

            if (self.app_data.has_binding(event, KeyCommand::SelectorRight)
                || event.is_in(MouseEventKind::RightClick, self.areas[1]))
                && self.resources.is_resources_selector_allowed()
            {
                self.res_selector
                    .show_selected_uid(self.resources.table.get_kind_for_selector().as_str());
                return ResponseEvent::Handled;
            }
        }

        self.resources.process_event(event)
    }

    /// Allows all views to do some computations on every app tick.
    pub fn process_ticks(&mut self) -> ResponseEvent {
        if let Some(view_result) = self.view.as_mut().map(|view| view.process_tick()) {
            if view_result == ResponseEvent::Cancelled {
                self.view = None;
                return ResponseEvent::Handled;
            }

            view_result
        } else {
            ResponseEvent::Handled
        }
    }

    /// Processes connection event.
    pub fn process_connection_event(&mut self, is_connected: bool) {
        if is_connected {
            self.footer().reset("100_disconnected");
        } else {
            self.footer().set_icon("100_disconnected", Some(''), IconKind::Error);
            self.ns_selector.hide();
            self.res_selector.hide();

            self.resources.process_disconnection();
            if let Some(view) = &mut self.view {
                view.process_disconnection();
            }
        }
    }

    /// Handles namespace change.
    pub fn handle_namespace_change(&mut self, namespace: Namespace) {
        self.resources.clear_header_scope(true);
        self.resources.set_namespace(namespace);
        if let Some(view) = &mut self.view {
            view.handle_namespace_change();
        }
    }

    /// Handles kind change.
    pub fn handle_kind_change(&mut self, resource_to_select: Option<String>) {
        self.resources.clear_header_scope(true);
        self.resources.set_next_highlight(resource_to_select);
        if let Some(view) = &mut self.view {
            view.handle_kind_change();
        }
    }

    /// Adds current resource to the previous resources stack.
    pub fn remember_current_resource(&mut self) {
        self.resources.remember_current_resource();
    }

    pub fn process_context_change(&mut self, context: String, namespace: Namespace, version: String, scope: Scope) {
        self.resources.set_resources_info(context, namespace, version, scope);
    }

    /// Resets all data for views.
    pub fn reset(&mut self) {
        self.resources.clear_list_data();
        self.ns_selector.select.items.clear();
        self.ns_selector.hide();
        self.res_selector.select.items.clear();
        self.res_selector.hide();
    }

    /// Clears resources list.
    pub fn clear_page_view(&mut self) {
        self.resources.clear_list_data();
    }

    /// Sets page view from resource scope.
    pub fn set_page_view(&mut self, scope: &Scope, is_all_namespaces: bool) {
        if *scope == Scope::Namespaced && is_all_namespaces {
            self.resources.set_view(ViewType::Full);
        } else {
            self.resources.set_view(ViewType::Compact);
        }
    }

    /// Forces scope for the resources header.
    pub fn force_header_scope(&mut self, scope: Option<Scope>) {
        self.resources.clear_header_scope(false);
        self.resources.table.header.set_scope(scope);
    }

    /// Shows delete resources dialog if anything is selected.
    pub fn ask_delete_resources(&mut self) {
        self.resources.ask_delete_resources();
    }

    /// Deletes resources that are currently selected on [`ResourcesView`].
    pub fn delete_resources(
        &mut self,
        propagation_policy: PropagationPolicy,
        terminate_immediately: bool,
        detach_finalizers: bool,
    ) {
        let resources = self.resources.table.list.table.get_selected_resources();
        let mut grouped: HashMap<&str, Vec<&ResourceItem>> = HashMap::new();
        for resource in resources {
            let namespace = resource.namespace.as_deref().unwrap_or_default();
            grouped.entry(namespace).or_default().push(resource);
        }

        for (namespace, resources) in grouped {
            let options = DeleteResourcesOptions {
                propagation_policy,
                terminate_immediately,
                detach_finalizers,
            };
            self.worker.borrow_mut().delete_resources(
                resources.iter().map(|r| (r.name.clone(), r.uid.clone())).collect(),
                namespace.into(),
                &self.resources.get_kind(),
                options,
                self.footer.get_transmitter(),
            );
        }

        self.resources.deselect_all();
        self.footer
            .transmitter()
            .show_info("Selected resources marked for deletion", 3_000);
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: &[NamedContext]) {
        self.resources.show_contexts_list(list);
    }

    /// Displays a list of available themes to choose from.
    pub fn show_themes_list(&mut self, list: Vec<std::path::PathBuf>) {
        self.resources.show_themes_list(list);
    }

    /// Displays a list of known namespaces to choose from.
    pub fn show_namespaces_list(&mut self) {
        let namespaces = self.ns_selector.select.items.get_as_actions();
        self.resources.show_namespaces_list(namespaces);
    }

    /// Shows logs for the specified container or multiple containers if `containers` are provided.
    pub fn show_logs(&mut self, resource: &ResourceRef, containers: Option<Vec<ResourceTag>>, previous: bool) {
        let worker = self.worker.borrow();
        let Some(client) = worker.kubernetes_client() else {
            return;
        };

        let pods = match containers {
            Some(containers) if !containers.is_empty() => containers
                .into_iter()
                .map(|container| {
                    ContainerRef::new(
                        resource.name.clone().unwrap_or_default(),
                        resource.namespace.clone(),
                        Some(container),
                    )
                })
                .collect(),
            _ => vec![ContainerRef::simple(
                resource.name.clone().unwrap_or_default(),
                resource.namespace.clone(),
                resource.container.clone(),
            )],
        };

        let view = LogsView::new(
            Rc::clone(&self.app_data),
            Rc::clone(&self.worker),
            client,
            pods,
            previous,
            self.footer.get_transmitter(),
            self.workspace,
        );

        if let Ok(view) = view {
            self.view = Some(Box::new(view));
        }
    }

    /// Sends command to fetch resource's YAML to the background executor and opens empty YAML view.
    pub fn show_yaml(&mut self, command_id: Option<String>, resource: ResourceRef, is_new: bool, edit: bool) {
        let mut view = YamlView::new(
            Rc::clone(&self.app_data),
            Rc::clone(&self.worker),
            command_id,
            resource,
            self.footer.get_transmitter(),
            is_new,
            self.workspace,
        );
        if edit {
            view.switch_to_edit();
        }

        self.view = Some(Box::new(view));
    }

    /// Shows returned resource's template YAML in an already opened YAML view.
    pub fn new_yaml_result(&mut self, command_id: &str, result: Result<GetNewResourceYamlResult, GetNewResourceYamlError>) {
        self.handle_yaml_result(command_id, result, CommandResult::GetNewResourceYaml, "New YAML", true);
    }

    /// Shows returned resource's YAML in an already opened YAML view.
    pub fn show_yaml_result(&mut self, command_id: &str, result: Result<ResourceYamlResult, ResourceYamlError>) {
        self.handle_yaml_result(command_id, result, CommandResult::GetResourceYaml, "View YAML", true);
    }

    /// Process YAML patch result.
    pub fn create_yaml_result(&mut self, command_id: &str, result: Result<String, SetNewResourceYamlError>) {
        self.handle_yaml_result(command_id, result, CommandResult::SetNewResourceYaml, "Create YAML", false);
    }

    /// Process YAML patch result.
    pub fn edit_yaml_result(&mut self, command_id: &str, result: Result<String, SetResourceYamlError>) {
        self.handle_yaml_result(command_id, result, CommandResult::SetResourceYaml, "Patch YAML", false);
    }

    fn handle_yaml_result<R, E, F>(&mut self, command_id: &str, result: Result<R, E>, wrap: F, error_msg: &str, close: bool)
    where
        E: std::fmt::Display,
        F: FnOnce(Result<R, E>) -> CommandResult,
    {
        if self.view.as_ref().is_some_and(|v| !v.command_id_match(command_id)) {
            return;
        }

        if let Err(error) = result {
            let msg = format!("{error_msg} error: {error}");
            tracing::warn!("{}", msg);
            self.footer.transmitter().show_error(msg, DEFAULT_ERROR_DURATION);
            if close {
                self.view = None;
            }
        } else if let Some(view) = &mut self.view {
            view.process_command_result(wrap(result));
        }
    }

    /// Opens shell for the specified container.
    pub fn open_shell(&mut self, resource: ResourceRef) {
        if let Some(client) = self.worker.borrow().kubernetes_client() {
            let view = ShellView::new(
                self.worker.borrow().runtime_handle().clone(),
                Rc::clone(&self.app_data),
                client,
                resource.name.unwrap_or_default(),
                resource.namespace,
                resource.container,
                self.footer.get_transmitter(),
            );
            self.view = Some(Box::new(view));
            self.footer().hide_hint();
        }
    }

    /// Displays a list of available forward ports for a container to choose from.
    pub fn show_ports_list(&mut self, list: &[Port]) {
        self.resources.show_ports_list(list);
    }

    /// Shows port forwards view.
    pub fn show_port_forwards(&mut self) {
        let view = ForwardsView::new(
            Rc::clone(&self.app_data),
            Rc::clone(&self.worker),
            self.footer.get_transmitter(),
        );
        self.view = Some(Box::new(view));
    }

    /// Updates footer message history pane hint with current key binding.
    pub fn set_message_history_hint(&mut self) {
        let copy_key = self.app_data.get_key_name(KeyCommand::ContentCopy).to_ascii_uppercase();
        let hint = format!(" Select message, then press ␝{copy_key}␝ to copy to clipboard  ");
        self.footer.set_message_history_hint(hint);
    }

    fn copy_footer_message(&self) {
        if let Some(message) = self.footer.get_highlighted_history_message() {
            if let Some(clipboard) = &mut self.app_data.borrow_mut().clipboard
                && clipboard.set_text(message).is_ok()
            {
                self.footer.transmitter().show_info("Message copied to clipboard", 3_000);
            } else {
                self.footer
                    .transmitter()
                    .show_error("Unable to access clipboard functionality", 5_000);
            }
        }
    }
}

fn set_command_palette_hint(footer_tx: &NotificationSink, app_data: &SharedAppData) {
    let command_palette_key = app_data.get_key_name(KeyCommand::CommandPaletteOpen).to_ascii_uppercase();
    footer_tx.show_hint(format!(" Press ␝{command_palette_key}␝ to open command palette"));
}
