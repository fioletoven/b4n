use anyhow::Result;
use kube::discovery::Scope;
use std::{cell::RefCell, rc::Rc};
use tracing::warn;

use crate::{
    kubernetes::{Kind, NAMESPACES, Namespace},
    ui::{
        ResponseEvent, Tui, TuiEvent, ViewType,
        theme::Theme,
        views::{LogsView, ResourcesView, ShellView, View, YamlView},
        widgets::{Footer, FooterMessage},
    },
};

use super::{
    AppData, BgWorker, BgWorkerError, Config, ConfigWatcher, History, KubernetesClientManager, ResourceRef, SharedAppData,
    SharedBgWorker,
    commands::{
        Command, CommandResult, KubernetesClientError, KubernetesClientResult, ListKubeContextsCommand, ResourceYamlError,
        ResourceYamlResult,
    },
};

/// Application execution flow.
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionFlow {
    Continue,
    Stop,
}

/// Main application object that orchestrates terminal, UI widgets and background workers.
pub struct App {
    data: SharedAppData,
    tui: Tui,
    resources: ResourcesView,
    view: Option<Box<dyn View>>,
    footer: Footer,
    worker: SharedBgWorker,
    config_watcher: ConfigWatcher<Config>,
    history_watcher: ConfigWatcher<History>,
    theme_watcher: ConfigWatcher<Theme>,
    client_manager: KubernetesClientManager,
}

impl App {
    /// Creates new [`App`] instance.
    pub fn new(config: Config, history: History, theme: Theme) -> Result<Self> {
        let theme_path = config.theme_path();
        let data = Rc::new(RefCell::new(AppData::new(config, history, theme)));
        let footer = Footer::new(Rc::clone(&data));
        let worker = Rc::new(RefCell::new(BgWorker::new(footer.get_messages_sender())));
        let resources = ResourcesView::new(Rc::clone(&data), Rc::clone(&worker));
        let client_manager = KubernetesClientManager::new(Rc::clone(&data), Rc::clone(&worker), footer.get_messages_sender());

        Ok(Self {
            data,
            tui: Tui::new()?,
            resources,
            view: None,
            footer,
            worker,
            config_watcher: Config::watcher(),
            history_watcher: History::watcher(),
            theme_watcher: ConfigWatcher::new(theme_path),
            client_manager,
        })
    }

    /// Starts app with initial data.
    pub fn start(&mut self, context: String, kind: Kind, namespace: Namespace) -> Result<()> {
        self.client_manager
            .request_new_client(context.clone(), kind, namespace.clone());
        self.resources
            .set_resources_info(context, namespace, String::default(), Scope::Cluster);
        self.config_watcher.start()?;
        self.history_watcher.start()?;
        self.theme_watcher.start()?;
        self.tui.enter_terminal()?;

        Ok(())
    }

    /// Cancels all app tasks.
    pub fn cancel(&mut self) {
        self.worker.borrow_mut().cancel_all();
        self.config_watcher.cancel();
        self.history_watcher.cancel();
        self.theme_watcher.cancel();
        self.tui.cancel();
    }

    /// Stops app.
    pub fn stop(&mut self) -> Result<()> {
        self.worker.borrow_mut().stop_all();
        self.config_watcher.stop();
        self.history_watcher.stop();
        self.theme_watcher.stop();
        self.tui.exit_terminal()?;

        Ok(())
    }

    /// Process all waiting events.
    pub fn process_events(&mut self) -> Result<ExecutionFlow> {
        if let Some(config) = self.config_watcher.try_next() {
            self.theme_watcher.change_file(config.theme_path())?;
            self.data.borrow_mut().config = config;
        }

        if let Some(history) = self.history_watcher.try_next() {
            self.data.borrow_mut().history = history;
        }

        if let Some(theme) = self.theme_watcher.try_next() {
            self.data.borrow_mut().theme = theme;
        }

        self.process_commands_results();
        self.process_connection_events();
        self.update_lists();

        if let Some(ResponseEvent::Cancelled) = self.process_view_events() {
            self.view = None;
        }

        while let Ok(event) = self.tui.event_rx.try_recv() {
            if self.process_event(event)? == ResponseEvent::ExitApplication {
                return Ok(ExecutionFlow::Stop);
            }
        }

        Ok(ExecutionFlow::Continue)
    }

    /// Draws UI page on terminal frame.
    pub fn draw_frame(&mut self) -> Result<()> {
        self.tui.terminal.draw(|frame| {
            let layout = Footer::get_layout(frame.area());
            self.footer.draw(frame, layout[1]);

            if let Some(view) = &mut self.view {
                view.draw(frame, layout[0]);
            } else {
                self.resources.draw(frame, layout[0]);
            }
        })?;

        Ok(())
    }

    /// Updates page lists with observed resources.
    fn update_lists(&mut self) {
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

    /// Process TUI event.
    fn process_event(&mut self, event: TuiEvent) -> Result<ResponseEvent> {
        if let Some(view) = &mut self.view {
            match view.process_event(event) {
                ResponseEvent::ExitApplication => return Ok(ResponseEvent::ExitApplication),
                ResponseEvent::Cancelled => self.view = None,
                _ => (),
            }
        } else {
            match self.resources.process_event(event) {
                ResponseEvent::ExitApplication => return Ok(ResponseEvent::ExitApplication),
                ResponseEvent::Change(kind, namespace) => self.change(kind.into(), namespace.into())?,
                ResponseEvent::ChangeKind(kind) => self.change_kind(kind.into(), None)?,
                ResponseEvent::ChangeKindAndSelect(kind, to_select) => self.change_kind(kind.into(), to_select)?,
                ResponseEvent::ChangeNamespace(namespace) => self.change_namespace(namespace.into())?,
                ResponseEvent::ViewContainers(pod_name, pod_namespace) => self.view_containers(pod_name, pod_namespace.into())?,
                ResponseEvent::ViewNamespaces => self.view_namespaces()?,
                ResponseEvent::ListKubeContexts => self.list_kube_contexts(),
                ResponseEvent::ChangeContext(context) => self.request_kubernetes_client(context),
                ResponseEvent::AskDeleteResources => self.resources.ask_delete_resources(),
                ResponseEvent::DeleteResources => self.delete_resources(),
                ResponseEvent::ViewYaml(resource, namespace, decode) => self.request_yaml(resource, namespace, decode),
                ResponseEvent::ViewLogs(pod_name, pod_namespace, pod_container) => {
                    self.view_logs(pod_name, pod_namespace, pod_container, false);
                },
                ResponseEvent::ViewPreviousLogs(pod_name, pod_namespace, pod_container) => {
                    self.view_logs(pod_name, pod_namespace, pod_container, true);
                },
                ResponseEvent::OpenShell(pod_name, pod_namespace, pod_container) => {
                    self.open_shell(pod_name, pod_namespace, pod_container);
                },
                _ => (),
            }
        }

        Ok(ResponseEvent::Handled)
    }

    /// Processes additional view events.
    fn process_view_events(&mut self) -> Option<ResponseEvent> {
        self.view.as_mut().map(|view| view.process_tick())
    }

    /// Processes results from commands execution.
    fn process_commands_results(&mut self) {
        let commands = self.worker.borrow_mut().get_all_waiting_results();
        for command in commands {
            match command.result {
                CommandResult::ContextsList(list) => self.resources.show_contexts_list(list),
                CommandResult::KubernetesClient(result) => self.change_client(&command.id, result),
                CommandResult::ResourceYaml(result) => self.show_yaml(&command.id, result),
            }
        }
    }

    /// Processes connection events.
    fn process_connection_events(&mut self) {
        self.data.borrow_mut().is_connected = !self.worker.borrow().has_errors();
        self.client_manager.process_request_overdue();
        if self.client_manager.should_process_disconnection() {
            self.resources.process_disconnection();
            if let Some(view) = &mut self.view {
                view.process_disconnection();
            }
        }
    }

    /// Changes observed resources namespace and kind.
    fn change(&mut self, kind: Kind, namespace: Namespace) -> Result<(), BgWorkerError> {
        if !self.data.borrow().current.is_namespace_equal(&namespace) || self.data.borrow().current.kind != kind {
            self.resources.set_namespace(namespace.clone());
            let resource = ResourceRef::new(kind.clone(), namespace.clone());
            let scope = self.worker.borrow_mut().restart(resource)?;
            self.process_resources_change(Some(kind.into()), Some(namespace.into()), Some(scope));
        }

        Ok(())
    }

    /// Changes observed resources kind, optionally selects one of them.\
    /// **Note** that it selects current namespace if the resource kind is `namespaces`.
    fn change_kind(&mut self, kind: Kind, to_select: Option<String>) -> Result<(), BgWorkerError> {
        if self.data.borrow().current.kind != kind {
            let namespace = self.data.borrow().current.get_namespace();
            let scope = self.worker.borrow_mut().restart_new_kind(kind.clone(), namespace)?;
            if to_select.is_none() && kind.as_str() == NAMESPACES {
                let to_select: Option<String> = Some(self.data.borrow().current.namespace.as_str().into());
                self.resources.highlight_next(to_select);
            } else {
                self.resources.highlight_next(to_select);
            }
            self.process_resources_change(Some(kind.into()), None, Some(scope));
        }

        Ok(())
    }

    /// Changes namespace for observed resources.
    fn change_namespace(&mut self, namespace: Namespace) -> Result<(), BgWorkerError> {
        if !self.data.borrow().current.is_namespace_equal(&namespace) {
            self.process_resources_change(None, Some(namespace.clone().into()), None);
            self.resources.set_namespace(namespace.clone());
            self.worker.borrow_mut().restart_new_namespace(namespace)?;
        }

        Ok(())
    }

    /// Changes observed resources to `containers` for a specified `pod`.
    fn view_containers(&mut self, pod_name: String, pod_namespace: Namespace) -> Result<(), BgWorkerError> {
        self.resources.clear_list_data();
        self.resources.set_view(ViewType::Compact);
        self.worker.borrow_mut().restart_containers(pod_name, pod_namespace)?;

        Ok(())
    }

    /// Changes observed resources kind to `namespaces`.
    fn view_namespaces(&mut self) -> Result<(), BgWorkerError> {
        self.change_kind(NAMESPACES.into(), None)?;

        Ok(())
    }

    /// Runs command to list kube contexts from the current config.
    fn list_kube_contexts(&mut self) {
        let kube_config_path = self.data.borrow().history.kube_config_path().map(String::from);
        self.worker
            .borrow_mut()
            .run_command(Command::ListKubeContexts(ListKubeContextsCommand { kube_config_path }));
    }

    /// Changes kubernetes client to the new one.
    fn change_client(&mut self, command_id: &str, result: Result<KubernetesClientResult, KubernetesClientError>) {
        if let Some(result) = self.client_manager.process_result(command_id, result) {
            let context = result.client.context().to_owned();
            let version = result.client.k8s_version().to_owned();
            let resource = ResourceRef::new(result.kind.clone(), result.namespace.clone());

            let scope = self.worker.borrow_mut().start(result.client, result.discovery, resource);
            if let Ok(scope) = scope {
                self.resources
                    .set_resources_info(context, result.namespace.clone(), version, scope.clone());
                self.process_resources_change(Some(result.kind.into()), Some(result.namespace.into()), Some(scope));
            }
        }
    }

    /// Deletes resources that are currently selected on [`ResourcesView`].
    fn delete_resources(&mut self) {
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

    /// Performs all necessary actions needed when resources view changes.\
    /// **Note** that this means the resource list will change soon.
    fn process_resources_change(&mut self, kind: Option<String>, namespace: Option<String>, scope: Option<Scope>) {
        self.resources.clear_list_data();
        self.update_history_data(kind, namespace);
        if let Some(scope) = scope {
            self.set_page_view(&scope);
        }
    }

    /// Updates `kind` and `namespace` in the app history data and saves it to a file.
    fn update_history_data(&mut self, kind: Option<String>, namespace: Option<String>) {
        let context = { self.data.borrow().current.context.clone() };
        self.data
            .borrow_mut()
            .history
            .create_or_update_context(context, kind, namespace);

        self.history_watcher.skip_next();
        self.worker.borrow_mut().save_history(self.data.borrow().history.clone());
    }

    /// Sets page view from resource scope.
    fn set_page_view(&mut self, result: &Scope) {
        if *result == Scope::Cluster {
            self.resources.set_view(ViewType::Compact);
        } else if self.data.borrow().current.is_all_namespace() {
            self.resources.set_view(ViewType::Full);
        }
    }

    /// Requests new kubernetes client with configured kind and namespace.
    fn request_kubernetes_client(&mut self, context: String) {
        if self.data.borrow().current.context == context {
            return;
        }

        self.client_manager.erase_request(true);
        self.worker.borrow_mut().stop();

        let (kind, namespace) = self.data.borrow().get_namespaced_resource_from_config(&context);
        self.resources.reset();
        self.resources
            .set_resources_info(context.clone(), namespace.clone(), String::default(), Scope::Cluster);

        self.client_manager.request_new_client(context, kind, namespace);
    }

    /// Sends command to fetch resource's YAML to the background executor.
    fn request_yaml(&mut self, resource: String, namespace: String, decode: bool) {
        let command_id = self.worker.borrow_mut().get_yaml(
            resource.clone(),
            namespace.clone().into(),
            &self.resources.get_kind(),
            self.data.borrow().get_syntax_data(),
            decode,
        );

        self.view = Some(Box::new(YamlView::new(
            Rc::clone(&self.data),
            Rc::clone(&self.worker),
            command_id,
            resource,
            namespace.into(),
            self.resources.get_kind(),
            self.footer.get_messages_sender(),
        )));
    }

    /// Shows returned resource's YAML in a separate view.
    fn show_yaml(&mut self, command_id: &str, result: Result<ResourceYamlResult, ResourceYamlError>) {
        if self.view.as_ref().is_some_and(|v| !v.command_id_match(command_id)) {
            return;
        }

        if let Err(error) = result {
            self.view = None;
            let msg = format!("View YAML error: {error}");
            warn!("{}", msg);
            self.footer.send_message(FooterMessage::error(msg, 0));
        } else if let Some(view) = &mut self.view {
            view.process_command_result(CommandResult::ResourceYaml(result));
        }
    }

    /// Shows logs for the specified container.
    fn view_logs(&mut self, pod_name: String, pod_namespace: String, pod_container: Option<String>, previous: bool) {
        if let Some(client) = self.worker.borrow().kubernetes_client() {
            if let Ok(view) = LogsView::new(
                Rc::clone(&self.data),
                client,
                pod_name,
                pod_namespace.into(),
                pod_container,
                previous,
            ) {
                self.view = Some(Box::new(view));
            }
        }
    }

    /// Opens shell for the specified container.
    fn open_shell(&mut self, pod_name: String, pod_namespace: String, pod_container: Option<String>) {
        if let Some(client) = self.worker.borrow().kubernetes_client() {
            let view = ShellView::new(
                Rc::clone(&self.data),
                client,
                pod_name,
                pod_namespace.into(),
                pod_container,
                self.footer.get_messages_sender(),
            );
            self.view = Some(Box::new(view));
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
