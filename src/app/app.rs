use anyhow::Result;
use kube::discovery::Scope;
use std::{cell::RefCell, rc::Rc, time::Instant};

use crate::{
    kubernetes::{Namespace, NAMESPACES},
    ui::{
        views::{ResourcesView, View, YamlView},
        ResponseEvent, Tui, TuiEvent, ViewType,
    },
};

use super::{
    commands::{
        Command, CommandResult, KubernetesClientError, KubernetesClientResult, ListKubeContextsCommand,
        NewKubernetesClientCommand,
    },
    AppData, BgWorker, BgWorkerError, Config, ConfigWatcher, SharedAppData,
};

/// Application execution flow.
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionFlow {
    Continue,
    Stop,
}

/// Application connecting info.
struct AppConnectingInfo {
    request_time: Instant,
    request_id: Option<String>,
    context: String,
    kind: String,
    namespace: Namespace,
}

impl AppConnectingInfo {
    /// Returns `true` if request match the specified ID.
    pub fn request_match(&self, request_id: &str) -> bool {
        self.request_id.as_deref().is_some_and(|id| id == request_id)
    }

    /// Returns `true` if there is no request pending and last request was more than 30 seconds ago.
    pub fn is_overdue(&self) -> bool {
        self.request_id.is_none() && self.request_time.elapsed().as_secs() > 30
    }
}

/// Main application object that orchestrates terminal, UI widgets and background workers.
pub struct App {
    data: SharedAppData,
    tui: Tui,
    resources: ResourcesView,
    view: Option<Box<dyn View>>,
    worker: BgWorker,
    watcher: ConfigWatcher,
    connecting: Option<AppConnectingInfo>,
    disconnect_processed: bool,
}

impl App {
    /// Creates new [`App`] instance.
    pub fn new(config: Config) -> Result<Self> {
        let data = Rc::new(RefCell::new(AppData::new(config)));
        let resources = ResourcesView::new(Rc::clone(&data));

        Ok(Self {
            data,
            tui: Tui::new()?,
            resources,
            view: None,
            worker: BgWorker::default(),
            watcher: Config::watcher(),
            connecting: None,
            disconnect_processed: false,
        })
    }

    /// Starts app with initial data.
    pub async fn start(&mut self, context: String, kind: String, namespace: Namespace) -> Result<()> {
        self.connecting = Some(self.new_kubernetes_client(context.clone(), kind, namespace.clone()));
        self.resources
            .set_resources_info(context, namespace, String::default(), Scope::Cluster);
        self.watcher.start()?;
        self.tui.enter_terminal()?;

        Ok(())
    }

    /// Cancels all app tasks.
    pub fn cancel(&mut self) {
        self.worker.cancel_all();
        self.watcher.cancel();
        self.tui.cancel();
    }

    /// Stops app.
    pub fn stop(&mut self) -> Result<()> {
        self.worker.stop_all();
        self.watcher.stop();
        self.tui.exit_terminal()?;

        Ok(())
    }

    /// Process all waiting events.
    pub fn process_events(&mut self) -> Result<ExecutionFlow> {
        if let Some(config) = self.watcher.try_next() {
            self.data.borrow_mut().config = config;
        }

        self.process_commands_results();

        self.process_connection_events();

        while let Ok(event) = self.tui.event_rx.try_recv() {
            if self.process_event(event)? == ResponseEvent::ExitApplication {
                return Ok(ExecutionFlow::Stop);
            }
        }

        Ok(ExecutionFlow::Continue)
    }

    /// Draws UI page on terminal frame.
    pub fn draw_frame(&mut self) -> Result<()> {
        self.update_lists();

        self.tui.terminal.draw(|frame| {
            if let Some(view) = &mut self.view {
                view.draw(frame);
            } else {
                self.resources.draw(frame);
            }
        })?;

        Ok(())
    }

    /// Updates page lists with observed resources.
    fn update_lists(&mut self) {
        if self.worker.update_discovery_list() {
            self.resources.update_kinds_list(self.worker.get_kinds_list());
        }

        self.resources.update_namespaces_list(self.worker.namespaces.try_next());
        self.resources.update_resources_list(self.worker.resources.try_next());

        self.data.borrow_mut().is_connected = !self.worker.has_errors();
    }

    /// Process TUI event.
    fn process_event(&mut self, event: TuiEvent) -> Result<ResponseEvent> {
        if let Some(view) = &mut self.view {
            match view.process_event(event) {
                ResponseEvent::ExitApplication => return Ok(ResponseEvent::ExitApplication),
                ResponseEvent::Cancelled => self.view = None,
                _ => (),
            };
        } else {
            match self.resources.process_event(event) {
                ResponseEvent::ExitApplication => return Ok(ResponseEvent::ExitApplication),
                ResponseEvent::Change(kind, namespace) => self.change(kind, namespace.into())?,
                ResponseEvent::ChangeKind(kind) => self.change_kind(kind, None)?,
                ResponseEvent::ChangeNamespace(namespace) => self.change_namespace(namespace.into())?,
                ResponseEvent::ViewNamespaces => self.view_namespaces()?,
                ResponseEvent::ListKubeContexts => self.list_kube_contexts(),
                ResponseEvent::ChangeContext(context) => self.ask_new_kubernetes_client(context),
                ResponseEvent::AskDeleteResources => self.resources.ask_delete_resources(),
                ResponseEvent::DeleteResources => self.delete_resources(),
                ResponseEvent::ViewYaml(resource, namespace) => {
                    self.worker.get_yaml(resource, namespace.into(), self.resources.kind_plural())
                }
                _ => (),
            };
        }

        Ok(ResponseEvent::Handled)
    }

    /// Process results from commands execution.
    fn process_commands_results(&mut self) {
        while let Some(command) = self.worker.check_command_result() {
            match command.result {
                CommandResult::ContextsList(list) => self.resources.show_contexts_list(list),
                CommandResult::KubernetesClient(result) => self.change_client(command.id, result),
                CommandResult::ResourceYaml(result) => self.view = Some(Box::new(YamlView::new(&self.data.borrow(), result))),
            }
        }
    }

    /// Processes connection events.
    fn process_connection_events(&mut self) {
        if self.connecting.as_ref().is_some_and(|c| c.is_overdue()) {
            if let Some(connecting) = self.connecting.take() {
                self.connecting = Some(self.new_kubernetes_client(connecting.context, connecting.kind, connecting.namespace));
            }
        }

        if !self.data.borrow().is_connected || self.connecting.is_some() {
            if !self.disconnect_processed {
                self.disconnect_processed = true;
                self.resources.process_disconnection();
            }
        } else {
            self.disconnect_processed = false;
        }
    }

    /// Changes observed resources namespace and kind.
    fn change(&mut self, kind: String, namespace: Namespace) -> Result<(), BgWorkerError> {
        self.update_configuration(Some(kind.clone()), Some(namespace.clone().into()));
        self.resources.set_namespace(namespace.clone());
        let scope = self.worker.restart(kind, namespace)?;
        self.set_page_view(scope);

        Ok(())
    }

    /// Changes observed resources kind, optionally selects one of them.  
    /// **Note** that it selects current namespace if the resource kind is `namespaces`.
    fn change_kind(&mut self, kind: String, to_select: Option<String>) -> Result<(), BgWorkerError> {
        self.update_configuration(Some(kind.clone()), None);
        let namespace = self.data.borrow().current.namespace.clone();
        let showing_namespaces = to_select.is_none() && kind == NAMESPACES;
        let scope = self.worker.restart_new_kind(kind, namespace)?;
        if showing_namespaces {
            let to_select: Option<String> = Some(self.data.borrow().current.namespace.as_str().into());
            self.resources.highlight_next(to_select);
        } else {
            self.resources.highlight_next(to_select);
        }
        self.set_page_view(scope);

        Ok(())
    }

    /// Changes namespace for observed resources.
    fn change_namespace(&mut self, namespace: Namespace) -> Result<(), BgWorkerError> {
        self.update_configuration(None, Some(namespace.clone().into()));
        self.resources.set_namespace(namespace.clone());
        self.worker.restart_new_namespace(namespace)?;

        Ok(())
    }

    /// Changes observed resources kind to `namespaces`.
    fn view_namespaces(&mut self) -> Result<(), BgWorkerError> {
        self.change_kind(NAMESPACES.to_owned(), None)?;

        Ok(())
    }

    /// Runs command to list kube contexts from the current config.
    fn list_kube_contexts(&mut self) {
        let kube_config_path = self.data.borrow().config.kube_config_path().map(String::from);
        self.worker
            .run_command(Command::ListKubeContexts(ListKubeContextsCommand { kube_config_path }));
    }

    /// Changes kubernetes client to the new one.
    fn change_client(&mut self, command_id: String, result: Result<KubernetesClientResult, KubernetesClientError>) {
        if self.connecting.as_ref().is_some_and(|c| c.request_match(&command_id)) {
            if let Ok(result) = result {
                self.connecting = None;
                let context = result.client.context().to_owned();
                let version = result.client.k8s_version().to_owned();

                let scope = self
                    .worker
                    .start(result.client, result.discovery, result.kind.clone(), result.namespace.clone());

                if let Ok(scope) = scope {
                    self.resources
                        .set_resources_info(context, result.namespace.clone(), version, scope.clone());
                    self.update_configuration(Some(result.kind), Some(result.namespace.into()));

                    self.set_page_view(scope);
                }
            } else if let Some(connecting) = &mut self.connecting {
                connecting.request_id = None;
            }
        }
    }

    /// Deletes resources that are currently selected on [`HomePage`].
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
                .delete_resources(resources, namespace, self.resources.kind_plural());
        }

        self.resources.deselect_all();
    }

    /// Sets page view from resource scope.
    fn set_page_view(&mut self, result: Scope) {
        if result == Scope::Cluster {
            self.resources.set_view(ViewType::Compact);
        } else if self.data.borrow().current.namespace.is_all() {
            self.resources.set_view(ViewType::Full);
        }
    }

    /// Updates `kind` and `namespace` in the configuration and saves it to a file.
    fn update_configuration(&mut self, kind: Option<String>, namespace: Option<String>) {
        let context = { self.data.borrow().current.context.clone() };
        self.data
            .borrow_mut()
            .config
            .create_or_update_context(context, kind, namespace);

        self.watcher.skip_next();
        self.worker.save_configuration(self.data.borrow().config.clone());
    }

    /// Sends command to create new kubernetes client to the background executor.
    fn new_kubernetes_client(&mut self, context: String, kind: String, namespace: Namespace) -> AppConnectingInfo {
        let kube_config_path = self.data.borrow().config.kube_config_path().map(String::from);
        let cmd = NewKubernetesClientCommand::new(kube_config_path, context.clone(), kind.clone(), namespace.clone());
        AppConnectingInfo {
            request_id: Some(self.worker.run_command(Command::NewKubernetesClient(Box::new(cmd)))),
            request_time: Instant::now(),
            context,
            kind,
            namespace,
        }
    }

    /// Sends command to create new kubernetes client with configured kind and namespace.
    fn ask_new_kubernetes_client(&mut self, context: String) {
        if self.data.borrow().current.context == context {
            return;
        }

        if let Some(connecting) = &self.connecting {
            self.worker.cancel_command(connecting.request_id.as_deref());
        }

        self.worker.stop();

        let (kind, namespace) = self.data.borrow().get_namespaced_resource_from_config(&context);
        self.resources.reset();
        self.resources
            .set_resources_info(context.clone(), namespace.clone(), String::default(), Scope::Cluster);

        self.connecting = Some(self.new_kubernetes_client(context, kind, namespace));
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
