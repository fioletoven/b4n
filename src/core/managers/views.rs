use anyhow::Result;
use kube::{config::NamedContext, discovery::Scope};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::rc::Rc;

use crate::{
    core::{
        SharedAppData, SharedAppDataExt, SharedBgWorker,
        commands::{CommandResult, ResourceYamlError, ResourceYamlResult, SetResourceYamlError},
    },
    kubernetes::{
        Namespace, ResourceRef,
        kinds::KindsList,
        resources::{Port, ResourcesList},
    },
    ui::{
        KeyCommand, MouseEventKind, ResponseEvent, Responsive, Table, TuiEvent, ViewType,
        views::{ForwardsView, LogsView, ResourcesView, ShellView, View, YamlView},
        widgets::{Footer, FooterTx, IconKind, Position, SideSelect},
    },
};

pub struct ViewsManager {
    app_data: SharedAppData,
    worker: SharedBgWorker,
    resources: ResourcesView,
    ns_selector: SideSelect<ResourcesList>,
    res_selector: SideSelect<KindsList>,
    view: Option<Box<dyn View>>,
    footer: Footer,
    areas: Rc<[Rect]>,
}

impl ViewsManager {
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, resources: ResourcesView, footer: Footer) -> Self {
        let ns_selector = SideSelect::new(
            "NAMESPACE",
            Rc::clone(&app_data),
            ResourcesList::default(),
            Position::Left,
            ResponseEvent::ChangeNamespace,
            30,
        );
        let res_selector = SideSelect::new(
            "RESOURCE",
            Rc::clone(&app_data),
            KindsList::default(),
            Position::Right,
            ResponseEvent::ChangeKind,
            35,
        );

        Self {
            app_data,
            worker,
            resources,
            ns_selector,
            res_selector,
            view: None,
            footer,
            areas: Rc::default(),
        }
    }

    /// Returns footer transmitter.
    pub fn footer(&self) -> &FooterTx {
        self.footer.transmitter()
    }

    /// Updates page lists with observed resources.
    pub fn update_lists(&mut self) {
        let mut worker = self.worker.borrow_mut();

        // Wait for the CRD list to become ready before polling anything else.
        // This ensures that the header for the current resource (if it's a CR)
        // is shown only after all columns are known.
        if !worker.is_crds_list_ready() {
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
        self.resources
            .update_error_state(worker.has_errors() || worker.resources.has_error());
    }

    /// Draws visible views on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let layout = Footer::get_layout(frame.area());
        self.footer.draw(frame, layout[1]);

        if let Some(view) = &mut self.view {
            view.draw(frame, layout[0]);
        } else {
            self.resources.draw(frame, layout[0]);
        }

        self.draw_selectors(frame, layout[0]);
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

        self.areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.ns_selector.width()),
                Constraint::Fill(1),
                Constraint::Length(self.res_selector.width()),
            ])
            .split(area);
    }

    /// Processes single TUI event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.ns_selector.is_visible {
            let result = self.ns_selector.process_event(event);
            if let Some(view) = &mut self.view {
                view.handle_namespaces_selector_event(&result);
            }

            return result;
        }

        if self.res_selector.is_visible {
            let result = self.res_selector.process_event(event);
            if let Some(view) = &mut self.view {
                view.handle_resources_selector_event(&result);
            }

            return result;
        }

        if self.view.is_some() {
            self.process_view_event(event)
        } else {
            self.process_resources_event(event)
        }
    }

    fn process_view_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let Some(view) = &mut self.view else {
            return ResponseEvent::Handled;
        };

        if self.app_data.borrow().is_connected {
            if (self.app_data.has_binding(event, KeyCommand::SelectorLeft)
                || event.is_in(MouseEventKind::RightClick, self.areas[0]))
                && view.is_namespaces_selector_allowed()
            {
                self.ns_selector.show_selected(view.displayed_namespace());
                return ResponseEvent::Handled;
            }

            if (self.app_data.has_binding(event, KeyCommand::SelectorRight)
                || event.is_in(MouseEventKind::RightClick, self.areas[2]))
                && view.is_resources_selector_allowed()
            {
                self.res_selector.show_selected(self.resources.table.get_kind().as_str());
                return ResponseEvent::Handled;
            }
        }

        let response = view.process_event(event);
        if response == ResponseEvent::Cancelled {
            self.view = None;
        }

        response
    }

    fn process_resources_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.app_data.borrow().is_connected {
            if (self.app_data.has_binding(event, KeyCommand::SelectorLeft)
                || event.is_in(MouseEventKind::RightClick, self.areas[0]))
                && self.resources.is_namespaces_selector_allowed()
            {
                self.ns_selector
                    .show_selected(self.app_data.borrow().current.namespace.as_str());
                return ResponseEvent::Handled;
            }

            if (self.app_data.has_binding(event, KeyCommand::SelectorRight)
                || event.is_in(MouseEventKind::RightClick, self.areas[2]))
                && self.resources.is_resources_selector_allowed()
            {
                self.res_selector.show_selected_uid(self.resources.table.get_kind().as_str());
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
        self.resources.highlight_next(resource_to_select);
        if let Some(view) = &mut self.view {
            view.handle_kind_change();
        }
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
    pub fn set_page_view(&mut self, scope: &Scope) {
        if *scope == Scope::Cluster {
            self.resources.set_view(ViewType::Compact);
        } else if self.app_data.borrow().current.is_all_namespace() {
            self.resources.set_view(ViewType::Full);
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
    pub fn delete_resources(&mut self, force: bool) {
        let list = self.resources.get_selected_items();
        for key in list.keys() {
            let resources = list[key].iter().map(|r| (*r).to_owned()).collect();
            let namespace = if self.resources.scope() == &Scope::Cluster {
                Namespace::all()
            } else {
                Namespace::from((*key).to_owned())
            };
            self.worker
                .borrow_mut()
                .delete_resources(resources, namespace, &self.resources.get_kind(), force);
        }

        self.resources.deselect_all();
        self.footer
            .transmitter()
            .show_info(" Selected resources marked for deletion…", 2_000);
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: &[NamedContext]) {
        self.resources.show_contexts_list(list);
    }

    /// Displays a list of available themes to choose from.
    pub fn show_themes_list(&mut self, list: Vec<std::path::PathBuf>) {
        self.resources.show_themes_list(list);
    }

    /// Shows logs for the specified container.
    pub fn show_logs(&mut self, resource: ResourceRef, previous: bool) {
        if let Some(client) = self.worker.borrow().kubernetes_client() {
            let view = LogsView::new(
                Rc::clone(&self.app_data),
                Rc::clone(&self.worker),
                client,
                resource,
                previous,
                self.footer.get_transmitter(),
            );
            if let Ok(view) = view {
                self.view = Some(Box::new(view));
            }
        }
    }

    /// Sends command to fetch resource's YAML to the background executor and opens empty YAML view.
    pub fn show_yaml(&mut self, command_id: Option<String>, resource: ResourceRef) {
        self.view = Some(Box::new(YamlView::new(
            Rc::clone(&self.app_data),
            Rc::clone(&self.worker),
            command_id,
            resource,
            self.footer.get_transmitter(),
        )));
    }

    /// Shows returned resource's YAML in an already opened YAML view.
    pub fn show_yaml_result(&mut self, command_id: &str, result: Result<ResourceYamlResult, ResourceYamlError>) {
        if self.view.as_ref().is_some_and(|v| !v.command_id_match(command_id)) {
            return;
        }

        if let Err(error) = result {
            self.view = None;
            let msg = format!("View YAML error: {error}");
            tracing::warn!("{}", msg);
            self.footer.transmitter().show_error(msg, 0);
        } else if let Some(view) = &mut self.view {
            view.process_command_result(CommandResult::GetResourceYaml(result));
        }
    }

    /// Process YAML patch result.
    pub fn edit_yaml_result(&mut self, command_id: &str, result: Result<String, SetResourceYamlError>) {
        if self.view.as_ref().is_some_and(|v| !v.command_id_match(command_id)) {
            return;
        }

        match result {
            Ok(name) => {
                self.view = None;
                self.footer
                    .transmitter()
                    .show_info(format!(" '{name}' YAML saved successfully…"), 2_000);
            },
            Err(error) => {
                tracing::warn!("Patch YAML error: {error}");
                self.footer
                    .transmitter()
                    .show_error("Patching the resource with the specified YAML failed…", 0);
            },
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
}
