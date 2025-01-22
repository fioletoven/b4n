use anyhow::Result;
use kube::{
    api::ApiResource,
    discovery::{ApiCapabilities, Scope},
};
use std::{cell::RefCell, rc::Rc};

use crate::{
    kubernetes::{client::KubernetesClient, Namespace, NAMESPACES},
    ui::{pages::HomePage, ResponseEvent, Tui, TuiEvent, ViewType},
};

use super::{
    commands::{Command, CommandResult, KubernetesClientResult, ListKubeContextsCommand, NewKubernetesClientCommand},
    AppData, BgWorker, BgWorkerError, Config, ConfigWatcher, ContextInfo, SharedAppData,
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
    page: HomePage,
    worker: BgWorker,
    watcher: ConfigWatcher,
    expected_client_id: Option<String>,
}

impl App {
    /// Creates new [`App`] instance.
    pub fn new(config: Config) -> Result<Self> {
        let data = Rc::new(RefCell::new(AppData::new(config)));
        let page = HomePage::new(Rc::clone(&data));

        Ok(Self {
            data,
            tui: Tui::new()?,
            page,
            worker: BgWorker::default(),
            watcher: Config::watcher(),
            expected_client_id: None,
        })
    }

    /// Starts app with initial data.
    pub async fn start(&mut self, context: String, kind: String, namespace: Namespace) -> Result<()> {
        self.expected_client_id = Some(self.get_new_kubernetes_client(context.clone(), kind, namespace.clone()));

        self.page
            .set_resources_info(context, namespace, String::default(), Scope::Cluster);

        self.watcher.start()?;

        self.tui.enter_terminal()?;

        Ok(())
    }

    /// Changes kubernetes client to the received one and restarts all required background processes.
    fn restart(
        &mut self,
        client: KubernetesClient,
        discovery: Vec<(ApiResource, ApiCapabilities)>,
        kind: String,
        namespace: Namespace,
    ) {
        let context = client.context().to_owned();
        let version = client.k8s_version().to_owned();

        if let Ok(scope) = self.worker.start(client, discovery, kind.clone(), namespace.clone()) {
            self.page
                .set_resources_info(context, namespace.clone(), version, scope.clone());
            self.update_configuration(Some(kind), Some(namespace.into()));

            self.set_page_view(scope);
        }
    }

    /// Stops app.
    pub fn stop(&mut self) -> Result<()> {
        self.worker.stop_all();
        self.watcher.stop();
        self.tui.exit_terminal()?;

        Ok(())
    }

    /// Cancels all app tasks.
    pub fn cancel(&mut self) {
        self.worker.cancel_all();
        self.watcher.cancel();
        self.tui.cancel();
    }

    /// Process all waiting events.
    pub fn process_events(&mut self) -> Result<ExecutionFlow> {
        while let Ok(event) = self.tui.event_rx.try_recv() {
            if self.process_event(event)? == ResponseEvent::ExitApplication {
                return Ok(ExecutionFlow::Stop);
            }
        }

        if let Some(config) = self.watcher.try_next() {
            self.data.borrow_mut().config = config;
        }

        self.process_commands_results();

        Ok(ExecutionFlow::Continue)
    }

    /// Draws UI page on terminal frame.
    pub fn draw_frame(&mut self) -> Result<()> {
        self.update_lists();

        self.tui.terminal.draw(|frame| {
            self.page.draw(frame);
        })?;

        Ok(())
    }

    /// Updates page lists with observed resources.
    fn update_lists(&mut self) {
        if self.worker.update_discovery_list() {
            self.page.update_kinds_list(self.worker.get_kinds_list());
        }

        self.page.update_namespaces_list(self.worker.namespaces.try_next());
        self.page.update_resources_list(self.worker.resources.try_next());

        self.data.borrow_mut().is_connected = !self.worker.has_errors();
    }

    /// Process TUI event.
    fn process_event(&mut self, event: TuiEvent) -> Result<ResponseEvent> {
        match self.page.process_event(event) {
            ResponseEvent::ExitApplication => return Ok(ResponseEvent::ExitApplication),
            ResponseEvent::Change(kind, namespace) => self.change(kind, namespace.into())?,
            ResponseEvent::ChangeKind(kind) => self.change_kind(kind, None)?,
            ResponseEvent::ChangeNamespace(namespace) => self.change_namespace(namespace.into())?,
            ResponseEvent::ViewNamespaces(selected_namespace) => self.view_namespaces(selected_namespace)?,
            ResponseEvent::ListKubeContexts => {
                self.worker.run_command(Command::ListKubeContexts(ListKubeContextsCommand {}));
            }
            ResponseEvent::ChangeContext(context) => self.ask_new_kubernetes_client(context),
            ResponseEvent::AskDeleteResources => self.page.ask_delete_resources(),
            ResponseEvent::DeleteResources => self.delete_resources(),
            _ => (),
        };

        Ok(ResponseEvent::Handled)
    }

    /// Process results from commands execution.
    fn process_commands_results(&mut self) {
        while let Some(command) = self.worker.check_command_result() {
            match command.result {
                CommandResult::ContextsList(list) => self.page.show_contexts_list(list),
                CommandResult::KubernetesClient(result) => self.change_client(command.id, result),
            }
        }
    }

    /// Changes observed resources namespace and kind.
    fn change(&mut self, kind: String, namespace: Namespace) -> Result<(), BgWorkerError> {
        self.update_configuration(Some(kind.clone()), Some(namespace.clone().into()));
        if namespace.is_all() {
            self.page.set_namespace(namespace.clone(), ViewType::Full);
        } else {
            self.page.set_namespace(namespace.clone(), ViewType::Compact);
        }

        let scope = self.worker.restart(kind, namespace)?;
        self.set_page_view(scope);

        Ok(())
    }

    /// Changes observed resources kind, optionally selects one of them.
    fn change_kind(&mut self, kind: String, to_select: Option<String>) -> Result<(), BgWorkerError> {
        self.update_configuration(Some(kind.clone()), None);
        let namespace = self.data.borrow().current.namespace.clone();
        let scope = self.worker.restart_new_kind(kind, namespace)?;
        self.page.highlight_next(to_select);
        self.set_page_view(scope);

        Ok(())
    }

    /// Changes namespace for observed resources.
    fn change_namespace(&mut self, namespace: Namespace) -> Result<(), BgWorkerError> {
        self.update_configuration(None, Some(namespace.clone().into()));
        if namespace.is_all() {
            self.page.set_namespace(namespace.clone(), ViewType::Full);
        } else {
            self.page.set_namespace(namespace.clone(), ViewType::Compact);
        }

        self.worker.restart_new_namespace(namespace)?;
        Ok(())
    }

    /// Changes observed resources kind to `namespaces` and selects provided namespace.
    fn view_namespaces(&mut self, namespace_to_select: String) -> Result<(), BgWorkerError> {
        self.change_kind(NAMESPACES.to_owned(), Some(namespace_to_select))?;
        Ok(())
    }

    /// Changes kubernetes client to the new one.
    fn change_client(&mut self, client_id: String, result: KubernetesClientResult) {
        if self.expected_client_id.as_deref().is_some_and(|id| id == client_id) {
            self.expected_client_id = None;
            self.restart(result.client, result.discovery, result.kind, result.namespace);
        }
    }

    /// Deletes resources that are currently selected on [`HomePage`].
    fn delete_resources(&mut self) {
        let list = self.page.get_selected_items();
        for key in list.keys() {
            let resources = list[key].iter().map(|r| (*r).to_owned()).collect();
            let namespace = if self.page.scope() == &Scope::Cluster {
                Namespace::all()
            } else {
                Namespace::from((*key).to_owned())
            };
            self.worker.delete_resources(resources, namespace, self.page.kind_plural());
        }

        self.page.deselect_all();
    }

    /// Sets page view from resource scope.
    fn set_page_view(&mut self, result: Scope) {
        if result == Scope::Cluster {
            self.page.set_view(ViewType::Compact);
        } else if self.data.borrow().current.namespace.is_all() {
            self.page.set_view(ViewType::Full);
        }
    }

    /// Updates `kind` and `namespace` in the configuration and saves it to a file.
    fn update_configuration(&mut self, kind: Option<String>, namespace: Option<String>) {
        let index = { self.data.borrow().config.context_index(&self.data.borrow().current.context) };
        if let Some(index) = index {
            let context = &mut self.data.borrow_mut().config.contexts[index];
            context.update(kind, namespace);
        } else {
            let mut context = { ContextInfo::from(&self.data.borrow().current) };
            context.update(kind, namespace);
            self.data.borrow_mut().config.contexts.push(context);
        }

        {
            let context = { self.data.borrow().current.context.clone() };
            self.data.borrow_mut().config.current_context = Some(context);
        }

        self.watcher.skip_next();
        self.worker.save_configuration(self.data.borrow().config.clone());
    }

    /// Sends command to create new kubernetes client to the background executor.
    fn get_new_kubernetes_client(&mut self, context: String, kind: String, namespace: Namespace) -> String {
        let cmd = NewKubernetesClientCommand::new(context, kind, namespace);
        self.worker.run_command(Command::NewKubernetesClient(cmd))
    }

    /// Sends command to create new kubernetes client with configured kind and namespace.
    fn ask_new_kubernetes_client(&mut self, context: String) {
        if self.data.borrow().current.context == context {
            return;
        }

        let (kind, namespace) = self.get_namespaced_resoruce_from_config(&context);

        self.worker.cancel_command(self.expected_client_id.as_deref());
        self.worker.stop();
        self.page.reset();
        self.page
            .set_resources_info(context.clone(), namespace.clone(), String::default(), Scope::Cluster);

        self.expected_client_id = Some(self.get_new_kubernetes_client(context, kind, namespace));
    }

    /// Returns resource's `kind` and `namespace` from the configuration file.  
    /// **Note** that if provided `context` is not found in the configuration file, current context resource is used.
    fn get_namespaced_resoruce_from_config(&self, context: &str) -> (String, Namespace) {
        let data = self.data.borrow();
        let kind = data.config.get_kind(context);
        if kind.is_none() {
            (data.current.kind_plural.clone(), data.current.namespace.clone())
        } else {
            let namespace = data.config.get_namespace(context).unwrap_or_default();
            (kind.unwrap_or_default().to_owned(), namespace.into())
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
