use anyhow::Result;
use kube::discovery::Scope;
use std::{cell::RefCell, rc::Rc};

use crate::{
    kubernetes::{client::KubernetesClient, ALL_NAMESPACES, NAMESPACES},
    ui::{pages::HomePage, ResponseEvent, Tui, TuiEvent, ViewType},
};

use super::{AppData, BgObserverError, BgWorker, Config, ConfigWatcher, ContextInfo, SharedAppData};

/// Application execution flow
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionFlow {
    Continue,
    Stop,
}

/// Main application object that orchestrates terminal, UI widgets and background workers
pub struct App {
    data: SharedAppData,
    tui: Tui,
    page: HomePage,
    worker: BgWorker,
    watcher: ConfigWatcher,
}

impl App {
    /// Creates new [`App`] instance
    pub fn new(client: KubernetesClient, config: Config) -> Result<Self> {
        let data = Rc::new(RefCell::new(AppData::new(config)));
        let page = HomePage::new(Rc::clone(&data));

        Ok(Self {
            data,
            tui: Tui::new()?,
            page,
            worker: BgWorker::new(client),
            watcher: Config::watcher(),
        })
    }

    /// Starts app with initial resource data
    pub async fn start(&mut self, resource_name: String, resource_namespace: Option<String>) -> Result<()> {
        let namespace = resource_namespace.as_deref().unwrap_or(ALL_NAMESPACES).to_owned();
        let kind = resource_name.clone();
        let scope = self.worker.start(resource_name, resource_namespace).await?;
        self.page.set_resources_info(
            self.worker.context().to_owned(),
            namespace.clone(),
            self.worker.k8s_version().to_owned(),
            scope,
        );

        self.watcher.start()?;
        self.update_configuration(Some(kind), Some(namespace));

        // we need to force update kinds list here, as the worker.start() consumes the first event from BgDiscovery
        self.page.update_kinds_list(self.worker.get_kinds_list());

        self.tui.enter_terminal()?;

        Ok(())
    }

    /// Stops app
    pub fn stop(&mut self) -> Result<()> {
        self.worker.stop();
        self.watcher.stop();
        self.tui.exit_terminal()?;

        Ok(())
    }

    /// Cancels all app tasks
    pub fn cancel(&mut self) {
        self.worker.cancel();
        self.watcher.cancel();
        self.tui.cancel();
    }

    /// Process all waiting UI or file events
    pub fn process_events(&mut self) -> Result<ExecutionFlow> {
        while let Ok(event) = self.tui.event_rx.try_recv() {
            if self.process_event(event)? == ResponseEvent::ExitApplication {
                return Ok(ExecutionFlow::Stop);
            }
        }

        if let Some(config) = self.watcher.try_next() {
            self.data.borrow_mut().config = config;
        }

        Ok(ExecutionFlow::Continue)
    }

    /// Draws UI page on terminal frame
    pub fn draw_frame(&mut self) -> Result<()> {
        self.update_lists();

        self.tui.terminal.draw(|frame| {
            self.page.draw(frame);
        })?;

        Ok(())
    }

    /// Updates page lists with observed resources
    fn update_lists(&mut self) {
        if self.worker.update_discovery_list() {
            self.page.update_kinds_list(self.worker.get_kinds_list());
        }

        self.page.update_namespaces_list(self.worker.namespaces.try_next());
        self.page.update_resources_list(self.worker.resources.try_next());

        self.data.borrow_mut().is_connected = !self.worker.has_errors();
    }

    /// Process TUI event
    fn process_event(&mut self, event: TuiEvent) -> Result<ResponseEvent> {
        match self.page.process_event(event) {
            ResponseEvent::ExitApplication => return Ok(ResponseEvent::ExitApplication),
            ResponseEvent::Change(kind, namespace) => self.change(kind, namespace)?,
            ResponseEvent::ChangeKind(kind) => self.change_kind(kind, None)?,
            ResponseEvent::ChangeNamespace(namespace) => self.change_namespace(namespace)?,
            ResponseEvent::ViewNamespaces(selected_namespace) => self.view_namespaces(selected_namespace)?,
            ResponseEvent::AskDeleteResources => self.page.ask_delete_resources(),
            ResponseEvent::DeleteResources => self.delete_resources(),
            _ => (),
        };

        Ok(ResponseEvent::Handled)
    }

    /// Changes observed resources namespace and kind
    fn change(&mut self, kind: String, namespace: String) -> Result<(), BgObserverError> {
        self.update_configuration(Some(kind.clone()), Some(namespace.clone()));
        let scope = if namespace == ALL_NAMESPACES {
            self.page.set_namespace(namespace, ViewType::Full);
            self.worker.restart(kind, None)?
        } else {
            self.page.set_namespace(namespace.clone(), ViewType::Compact);
            self.worker.restart(kind, Some(namespace))?
        };

        self.set_page_view(scope);

        Ok(())
    }

    /// Changes observed resources kind, optionally selects one of them
    fn change_kind(&mut self, kind: String, to_select: Option<String>) -> Result<(), BgObserverError> {
        self.update_configuration(Some(kind.clone()), None);
        let scope = self.worker.restart_new_kind(kind)?;
        self.page.highlight_next(to_select);
        self.set_page_view(scope);

        Ok(())
    }

    /// Changes namespace for observed resources
    fn change_namespace(&mut self, namespace: String) -> Result<(), BgObserverError> {
        self.update_configuration(None, Some(namespace.clone()));
        if namespace == ALL_NAMESPACES {
            self.page.set_namespace(namespace, ViewType::Full);
            self.worker.restart_new_namespace(None)?;
        } else {
            self.page.set_namespace(namespace.clone(), ViewType::Compact);
            self.worker.restart_new_namespace(Some(namespace))?;
        }

        Ok(())
    }

    /// Changes observed resources kind to `namespaces` and selects provided namespace
    fn view_namespaces(&mut self, namespace_to_select: String) -> Result<(), BgObserverError> {
        self.change_kind(NAMESPACES.to_owned(), Some(namespace_to_select))?;
        Ok(())
    }

    /// Deletes resources that are currently selected on [`HomePage`]
    fn delete_resources(&mut self) {
        let list = self.page.get_selected_items();
        for key in list.keys() {
            let resources = list[key].iter().map(|r| (*r).to_owned()).collect();
            let namespace = if self.page.scope() == &Scope::Cluster {
                None
            } else {
                Some((*key).to_owned())
            };
            self.worker.delete_resources(resources, namespace, self.page.kind_plural());
        }

        self.page.deselect_all();
    }

    /// Sets page view from resource scope
    fn set_page_view(&mut self, result: Scope) {
        if result == Scope::Cluster {
            self.page.set_view(ViewType::Compact);
        } else if self.data.borrow().current.namespace == ALL_NAMESPACES {
            self.page.set_view(ViewType::Full);
        }
    }

    /// Updates `kind` and `namespace` in the configuration and saves it to a file
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
            self.data.borrow_mut().config.context = Some(self.worker.context().to_owned());
        }

        self.watcher.skip_next();
        self.worker.save_configuration(self.data.borrow().config.clone());
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
