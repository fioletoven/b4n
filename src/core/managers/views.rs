use std::rc::Rc;

use anyhow::Result;
use kube::discovery::Scope;

use crate::{
    core::{
        SharedAppData, SharedBgWorker,
        commands::{CommandResult, ResourceYamlError, ResourceYamlResult},
    },
    kubernetes::{Namespace, ResourceRef},
    ui::{
        ResponseEvent, TuiEvent, ViewType,
        views::{ForwardsView, LogsView, ResourcesView, ShellView, View, YamlView},
        widgets::{Footer, FooterMessage},
    },
};

pub struct ViewsManager {
    pub resources: ResourcesView,
    pub view: Option<Box<dyn View>>,
    pub footer: Footer,
    app_data: SharedAppData,
    worker: SharedBgWorker,
}

impl ViewsManager {
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, resources: ResourcesView, footer: Footer) -> Self {
        Self {
            app_data,
            resources,
            view: None,
            footer,
            worker,
        }
    }

    /// Updates page lists with observed resources.
    pub fn update_lists(&mut self) {
        let mut worker = self.worker.borrow_mut();
        if worker.update_discovery_list() {
            self.resources.update_kinds_list(worker.get_kinds_list());
        }

        while let Some(update_result) = worker.namespaces.try_next() {
            self.resources.update_namespaces_list(*update_result);
        }

        while let Some(update_result) = worker.resources.try_next() {
            self.resources.update_resources_list(*update_result);
        }
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
    }

    /// Processes single TUI event.
    pub fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        if let Some(view) = &mut self.view {
            let result = view.process_event(event);
            if result == ResponseEvent::Cancelled {
                self.view = None;
            }

            result
        } else {
            self.resources.process_event(event)
        }
    }

    /// Process all waiting events.
    pub fn process_events(&mut self) -> ResponseEvent {
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

    /// Processes disconnection state.
    pub fn process_disconnection(&mut self) {
        self.resources.process_disconnection();
        if let Some(view) = &mut self.view {
            view.process_disconnection();
        }
    }

    /// Processes namespace change.
    pub fn process_namespace_change(&mut self, namespace: Namespace) {
        self.resources.set_namespace(namespace);
    }

    /// Processes kind change.
    pub fn process_kind_change(&mut self, resource_to_select: Option<String>) {
        self.resources.highlight_next(resource_to_select);
    }

    pub fn process_context_change(&mut self, reset: bool, context: String, namespace: Namespace, version: String, scope: Scope) {
        if reset {
            self.resources.reset();
        }

        self.resources
            .set_resources_info(context.clone(), namespace.clone(), version, scope);
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

    /// Shows delete resources dialog if anything is selected.
    pub fn ask_delete_resources(&mut self) {
        self.resources.ask_delete_resources();
    }

    /// Deletes resources that are currently selected on [`ResourcesView`].
    pub fn delete_resources(&mut self) {
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
                .delete_resources(resources, namespace, &self.resources.get_kind());
        }

        self.resources.deselect_all();
        self.footer
            .send_message(FooterMessage::info(" Selected resources marked for deletion…", 1_500));
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: Vec<kube::config::NamedContext>) {
        self.resources.show_contexts_list(list);
    }

    /// Shows logs for the specified container.
    pub fn show_logs(&mut self, resource: ResourceRef, previous: bool) {
        if let Some(client) = self.worker.borrow().kubernetes_client() {
            if let Ok(view) = LogsView::new(
                Rc::clone(&self.app_data),
                client,
                resource.name.unwrap_or_default(),
                resource.namespace,
                resource.container,
                previous,
            ) {
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
            resource.name.unwrap_or_default(),
            resource.namespace,
            resource.kind,
            self.footer.get_messages_sender(),
        )));
    }

    /// Shows returned resource's YAML in an already opened YAML view.
    pub fn update_yaml(&mut self, command_id: &str, result: Result<ResourceYamlResult, ResourceYamlError>) {
        if self.view.as_ref().is_some_and(|v| !v.command_id_match(command_id)) {
            return;
        }

        if let Err(error) = result {
            self.view = None;
            let msg = format!("View YAML error: {error}");
            tracing::warn!("{}", msg);
            self.footer.send_message(FooterMessage::error(msg, 0));
        } else if let Some(view) = &mut self.view {
            view.process_command_result(CommandResult::ResourceYaml(result));
        }
    }

    /// Opens shell for the specified container.
    pub fn open_shell(&mut self, resource: ResourceRef) {
        if let Some(client) = self.worker.borrow().kubernetes_client() {
            let view = ShellView::new(
                Rc::clone(&self.app_data),
                client,
                resource.name.unwrap_or_default(),
                resource.namespace,
                resource.container,
                self.footer.get_messages_sender(),
            );
            self.view = Some(Box::new(view));
        }
    }

    /// Displays a list of available forward ports for a container to choose from.
    pub fn show_ports_list(&mut self, list: Vec<crate::kubernetes::resources::Port>) {
        self.resources.show_ports_list(list);
    }

    /// Shows port forwards view.
    pub fn show_port_forwards(&mut self) {
        let view = ForwardsView::new(
            Rc::clone(&self.app_data),
            Rc::clone(&self.worker),
            self.footer.get_messages_sender(),
        );
        self.view = Some(Box::new(view));
    }
}
