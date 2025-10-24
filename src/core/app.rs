use anyhow::Result;
use kube::discovery::Scope;
use std::{
    cell::RefCell,
    net::{IpAddr, SocketAddr},
    rc::Rc,
};
use tokio::runtime::Handle;

use crate::{
    core::{SharedAppDataExt, ViewsManager, commands::ListThemesCommand},
    kubernetes::{Kind, NAMESPACES, Namespace, ResourceRef},
    ui::{
        KeyBindings, KeyCommand, ResponseEvent, ScopeData, Tui, TuiEvent,
        theme::Theme,
        views::ResourcesView,
        widgets::{Footer, IconKind},
    },
};

use super::{
    AppData, BgWorker, BgWorkerError, Config, ConfigWatcher, History, KubernetesClientManager, SharedAppData, SharedBgWorker,
    commands::{Command, CommandResult, KubernetesClientError, KubernetesClientResult, ListKubeContextsCommand},
};

/// Application execution flow.
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionFlow {
    Continue,
    Stop,
}

#[derive(Clone, Copy, PartialEq)]
enum TrackFlow {
    Nothing,
    Add,
    Clear,
}

/// Main application object that orchestrates terminal, UI widgets and background workers.
pub struct App {
    data: SharedAppData,
    tui: Tui,
    runtime: Handle,
    worker: SharedBgWorker,
    config_watcher: ConfigWatcher<Config>,
    history_watcher: ConfigWatcher<History>,
    theme_watcher: ConfigWatcher<Theme>,
    client_manager: KubernetesClientManager,
    views_manager: ViewsManager,
}

impl App {
    /// Creates new [`App`] instance.
    pub fn new(runtime: Handle, config: Config, history: History, theme: Theme, allow_insecure: bool) -> Result<Self> {
        let is_mouse_enabled = config.mouse;
        let theme_path = config.theme_path();
        let data = Rc::new(RefCell::new(AppData::new(config, history, theme)));
        let footer = Footer::new(Rc::clone(&data));
        let worker = Rc::new(RefCell::new(BgWorker::new(
            runtime.clone(),
            footer.get_transmitter(),
            data.borrow().get_syntax_data(),
        )));
        let resources = ResourcesView::new(Rc::clone(&data), Rc::clone(&worker));
        let client_manager =
            KubernetesClientManager::new(Rc::clone(&data), Rc::clone(&worker), footer.get_transmitter(), allow_insecure);
        let views_manager = ViewsManager::new(Rc::clone(&data), Rc::clone(&worker), resources, footer);

        Ok(Self {
            data,
            tui: Tui::new(is_mouse_enabled)?,
            runtime: runtime.clone(),
            worker,
            config_watcher: Config::watcher(runtime.clone()),
            history_watcher: History::watcher(runtime.clone()),
            theme_watcher: ConfigWatcher::new(runtime, theme_path),
            client_manager,
            views_manager,
        })
    }

    /// Starts app with initial data.
    pub fn start(&mut self, context: String, kind: Kind, namespace: Namespace) -> Result<()> {
        self.client_manager
            .request_new_client(context.clone(), kind, namespace.clone());
        self.views_manager
            .process_context_change(context, namespace, String::default(), Scope::Cluster);
        self.config_watcher.start()?;
        self.history_watcher.start()?;
        self.theme_watcher.start()?;
        self.tui.enter_terminal(&self.runtime)?;
        self.update_mouse_icon();

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
            let mut data = self.data.borrow_mut();
            data.key_bindings = KeyBindings::default_with(config.key_bindings.clone());
            data.config = config;
        }

        if let Some(history) = self.history_watcher.try_next() {
            self.data.borrow_mut().history = history;
        }

        if let Some(theme) = self.theme_watcher.try_next() {
            self.data.borrow_mut().theme = theme;
        }

        self.process_commands_results();
        self.process_connection_events();
        self.views_manager.update_lists();
        if self.views_manager.process_ticks() == ResponseEvent::ExitApplication {
            return Ok(ExecutionFlow::Stop);
        }

        while let Ok(event) = self.tui.event_rx.try_recv() {
            match self.process_event(&event) {
                Ok(response) => {
                    if response == ResponseEvent::ExitApplication {
                        return Ok(ExecutionFlow::Stop);
                    }
                },
                Err(error) => {
                    self.views_manager.footer().show_error(error.to_string(), 0);
                },
            }
        }

        Ok(ExecutionFlow::Continue)
    }

    /// Draws UI page on a terminal frame.
    pub fn draw_frame(&mut self) -> Result<()> {
        self.tui.terminal.draw(|frame| {
            self.views_manager.draw(frame);
        })?;

        Ok(())
    }

    /// Processes single TUI event.
    fn process_event(&mut self, event: &TuiEvent) -> Result<ResponseEvent> {
        if self.data.has_binding(event, KeyCommand::ApplicationExit) {
            return Ok(ResponseEvent::ExitApplication);
        }

        if self.data.has_binding(event, KeyCommand::MouseSupportToggle) {
            let _ = self.tui.toggle_mouse_support();
            self.update_mouse_icon();
            return Ok(ResponseEvent::Handled);
        }

        match self.views_manager.process_event(event) {
            ResponseEvent::ExitApplication => return Ok(ResponseEvent::ExitApplication),
            ResponseEvent::Change(kind, namespace) => self.change(kind.into(), namespace.into(), None, TrackFlow::Clear)?,
            ResponseEvent::ChangeAndSelect(kind, namespace, to_select) => {
                self.change(kind.into(), namespace.into(), to_select, TrackFlow::Clear)?;
            },
            ResponseEvent::ChangeAndSelectPrev(kind, namespace, to_select) => {
                self.change(kind.into(), namespace.into(), to_select, TrackFlow::Nothing)?;
            },
            ResponseEvent::ChangeKind(kind) => self.change_kind(kind.into(), None)?,
            ResponseEvent::ChangeKindAndSelect(kind, to_select) => self.change_kind(kind.into(), to_select)?,
            ResponseEvent::ChangeNamespace(namespace) => self.change_namespace(namespace.into())?,
            ResponseEvent::ViewContainers(pod_name, pod_namespace) => self.view_containers(pod_name, pod_namespace.into())?,
            ResponseEvent::ViewInvolved(kind, namespace, to_select) => {
                self.view_involved(kind.into(), namespace.into(), to_select)?;
            },
            ResponseEvent::ViewScoped(kind, namespace, to_select, scope) => {
                self.view_scoped(kind.into(), namespace.into(), to_select, scope, TrackFlow::Add)?;
            },
            ResponseEvent::ViewScopedPrev(kind, namespace, to_select, scope) => {
                self.view_scoped(kind.into(), namespace.into(), to_select, scope, TrackFlow::Nothing)?;
            },
            ResponseEvent::ViewNamespaces => self.view_namespaces()?,
            ResponseEvent::ListKubeContexts => self.list_kube_contexts(),
            ResponseEvent::ListThemes => self.list_app_themes(),
            ResponseEvent::ListResourcePorts(resource) => self.worker.borrow_mut().list_resource_ports(resource),
            ResponseEvent::ChangeContext(context) => self.request_kubernetes_client(context),
            ResponseEvent::ChangeTheme(theme) => self.process_theme_change(theme),
            ResponseEvent::AskDeleteResources => self.views_manager.ask_delete_resources(),
            ResponseEvent::DeleteResources(force) => self.views_manager.delete_resources(force),
            ResponseEvent::ViewYaml(resource, decode) => self.request_yaml(resource, decode),
            ResponseEvent::ViewLogs(container) => self.views_manager.show_logs(container, false),
            ResponseEvent::ViewPreviousLogs(container) => self.views_manager.show_logs(container, true),
            ResponseEvent::OpenShell(container) => self.views_manager.open_shell(container),
            ResponseEvent::ShowPortForwards => self.views_manager.show_port_forwards(),
            ResponseEvent::PortForward(resource, to, from, address) => self.port_forward(resource, to, from, &address),
            _ => (),
        }

        Ok(ResponseEvent::Handled)
    }

    /// Processes results from commands execution.
    fn process_commands_results(&mut self) {
        let commands = self.worker.borrow_mut().get_all_waiting_results();
        for command in commands {
            match command.result {
                CommandResult::KubernetesClient(result) => self.change_client(&command.id, result),
                CommandResult::GetResourceYaml(result) => self.views_manager.show_yaml_result(&command.id, result),
                CommandResult::SetResourceYaml(result) => self.views_manager.edit_yaml_result(&command.id, result),
                CommandResult::ContextsList(list) => self.views_manager.show_contexts_list(&list),
                CommandResult::ThemesList(list) => self.views_manager.show_themes_list(list),
                CommandResult::ResourcePortsList(list) => self.views_manager.show_ports_list(&list),
            }
        }
    }

    /// Processes connection events.
    fn process_connection_events(&mut self) {
        self.data.borrow_mut().is_connected = !self.worker.borrow().has_errors();
        self.client_manager.process_request_overdue();
        if let Some(is_connected) = self.client_manager.get_connection_state_if_changed() {
            self.views_manager.process_connection_event(*is_connected);
        }
    }

    /// Changes observed resources namespace and kind, optionally selects one of the new kinds.
    fn change(
        &mut self,
        kind: Kind,
        namespace: Namespace,
        to_select: Option<String>,
        track: TrackFlow,
    ) -> Result<(), BgWorkerError> {
        let kind = self.worker.borrow().ensure_kind_is_plural(kind);
        if !self.data.borrow().current.is_namespace_equal(&namespace)
            || !self.data.borrow().current.is_kind_equal(&kind)
            || self.data.borrow().current.resource.filter.is_some()
        {
            if track == TrackFlow::Add {
                self.views_manager.remember_current_resource();
            } else if track == TrackFlow::Clear {
                self.data.borrow_mut().previous.clear();
            }

            self.views_manager.handle_kind_change(to_select);
            self.views_manager.handle_namespace_change(namespace.clone());
            let resource = ResourceRef::new(kind.clone(), namespace.clone());
            let scope = self.worker.borrow_mut().restart(resource)?;
            self.process_resources_change(Some(kind.into()), Some(namespace.into()), &scope);
        }

        Ok(())
    }

    /// Changes observed resources kind, optionally selects one of them.\
    /// **Note** that it selects current namespace if the resource kind is `namespaces`.
    fn change_kind(&mut self, kind: Kind, to_select: Option<String>) -> Result<(), BgWorkerError> {
        let kind = self.worker.borrow().ensure_kind_is_plural(kind);
        if !self.data.borrow().current.is_kind_equal(&kind) || self.data.borrow().current.resource.filter.is_some() {
            let namespace = self.data.borrow().current.get_namespace();
            let scope = self.worker.borrow_mut().restart_new_kind(kind.clone(), namespace)?;
            if to_select.is_none() && kind.as_str() == NAMESPACES {
                let to_select: Option<String> = Some(self.data.borrow().current.get_namespace().as_str().to_owned());
                self.views_manager.handle_kind_change(to_select);
            } else {
                self.views_manager.handle_kind_change(to_select);
            }
            self.process_resources_change(Some(kind.into()), None, &scope);
            self.data.borrow_mut().previous.clear();
        }

        Ok(())
    }

    /// Changes namespace for observed resources.
    fn change_namespace(&mut self, namespace: Namespace) -> Result<(), BgWorkerError> {
        if !self.data.borrow().current.is_namespace_equal(&namespace) {
            if self.data.borrow().is_constrained() && !self.data.borrow().current.resource.kind.is_namespaces() {
                let previous_kind = self.data.borrow().previous.last().map(|p| p.resource.kind.clone());
                if let Some(kind) = previous_kind {
                    let name = self
                        .data
                        .borrow()
                        .previous
                        .last()
                        .and_then(|p| p.highlighted())
                        .map(String::from);
                    self.change(kind, namespace, name, TrackFlow::Clear)?;
                }
            } else {
                self.update_history_data(None, Some(namespace.clone().into()));
                self.views_manager.handle_namespace_change(namespace.clone());
                if self.data.borrow().current.scope == Scope::Namespaced {
                    self.views_manager.clear_page_view();
                    self.worker.borrow_mut().restart_new_namespace(namespace)?;
                }
            }
        }

        Ok(())
    }

    /// Changes observed resources to `containers` for a specified `pod`.
    fn view_containers(&mut self, pod_name: String, pod_namespace: Namespace) -> Result<(), BgWorkerError> {
        self.views_manager.remember_current_resource();
        self.views_manager.clear_page_view();
        self.views_manager.set_page_view(&Scope::Cluster);
        self.views_manager.force_header_scope(Some(Scope::Namespaced));
        self.worker.borrow_mut().restart_containers(pod_name, pod_namespace)?;

        Ok(())
    }

    /// Changes observed resource to the involved object.
    fn view_involved(&mut self, kind: Kind, namespace: Namespace, to_select: Option<String>) -> Result<(), BgWorkerError> {
        self.change(kind, namespace, to_select, TrackFlow::Add)
    }

    /// Changes observed resource to the scoped one.
    fn view_scoped(
        &mut self,
        kind: Kind,
        namespace: Namespace,
        to_select: Option<String>,
        scope: ScopeData,
        track: TrackFlow,
    ) -> Result<(), BgWorkerError> {
        if !self.data.borrow().current.is_kind_equal(&kind) {
            if track == TrackFlow::Add {
                self.views_manager.remember_current_resource();
            }

            let resource = ResourceRef::filtered(kind, namespace, scope.filter);
            self.views_manager.handle_kind_change(to_select);
            self.views_manager.clear_page_view();
            self.views_manager.set_page_view(&scope.list);
            self.views_manager.force_header_scope(Some(scope.header));
            self.worker.borrow_mut().restart(resource)?;
        }

        Ok(())
    }

    /// Changes observed resources kind to `namespaces`.
    fn view_namespaces(&mut self) -> Result<(), BgWorkerError> {
        self.change_kind(NAMESPACES.into(), None)
    }

    /// Runs command to list kube contexts from the current config.
    fn list_kube_contexts(&mut self) {
        let kube_config_path = self.data.borrow().history.kube_config_path().map(String::from);
        self.worker
            .borrow_mut()
            .run_command(Command::ListKubeContexts(ListKubeContextsCommand { kube_config_path }));
    }

    /// Runs command to list themes from the themes directory.
    fn list_app_themes(&self) {
        self.worker.borrow_mut().run_command(Command::ListThemes(ListThemesCommand));
    }

    /// Changes kubernetes client to the new one.
    fn change_client(&mut self, command_id: &str, result: Result<KubernetesClientResult, KubernetesClientError>) {
        if let Some(result) = self.client_manager.process_result(command_id, result) {
            let context = result.client.context().to_owned();
            let version = result.client.k8s_version().to_owned();
            let resource = ResourceRef::new(result.kind.clone(), result.namespace.clone());

            let scope = self.worker.borrow_mut().start(result.client, result.discovery, resource);
            if let Ok(scope) = scope {
                self.views_manager
                    .process_context_change(context, result.namespace.clone(), version, scope.clone());
                self.process_resources_change(Some(result.kind.into()), Some(result.namespace.into()), &scope);
            }
        }
    }

    /// Performs all necessary actions needed when resources view changes.\
    /// **Note** that this means the resource list will change soon.
    fn process_resources_change(&mut self, kind: Option<String>, namespace: Option<String>, scope: &Scope) {
        self.views_manager.clear_page_view();
        self.update_history_data(kind, namespace);
        self.views_manager.set_page_view(scope);
    }

    /// Changes application theme.
    fn process_theme_change(&mut self, theme: String) {
        if self.data.borrow().config.theme != theme {
            let msg = format!("Theme changed to '{theme}'…");
            self.data.borrow_mut().config.theme = theme;
            let _ = self.theme_watcher.change_file(self.data.borrow().config.theme_path());
            self.config_watcher.skip_next();
            self.worker.borrow_mut().save_config(self.data.borrow().config.clone());
            self.views_manager.footer().show_info(msg, 0);
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

    /// Requests new kubernetes client with configured kind and namespace.
    fn request_kubernetes_client(&mut self, context: String) {
        if self.data.borrow().current.context == context {
            return;
        }

        self.client_manager.erase_request(true);
        self.worker.borrow_mut().stop();

        let (kind, namespace) = self.data.borrow().get_namespaced_resource_from_config(&context);
        self.views_manager.reset();
        self.views_manager
            .process_context_change(context.clone(), namespace.clone(), String::default(), Scope::Cluster);

        self.client_manager.request_new_client(context, kind, namespace);
    }

    /// Sends command to fetch resource's YAML to the background executor.
    fn request_yaml(&mut self, resource: ResourceRef, decode: bool) {
        let command_id = self.worker.borrow_mut().get_yaml(
            resource.name.clone().unwrap_or_default(),
            resource.namespace.clone(),
            &resource.kind,
            decode,
        );

        self.views_manager.show_yaml(command_id, resource);
    }

    /// Creates port forward task for the specified resource.
    fn port_forward(&mut self, resource: ResourceRef, container_port: u16, local_port: u16, local_address: &str) {
        if let Ok(ip_addr) = local_address.parse::<IpAddr>() {
            let address = SocketAddr::from((ip_addr, local_port));
            self.worker.borrow_mut().start_port_forward(resource, container_port, address);
        }
    }

    fn update_mouse_icon(&self) {
        let icon = if self.tui.is_mouse_enabled() { Some('󰍽') } else { None };
        self.views_manager.footer().set_icon("000_mouse", icon, IconKind::Default);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
