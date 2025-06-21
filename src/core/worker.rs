use anyhow::Result;
use kube::{
    api::ApiResource,
    discovery::{ApiCapabilities, Scope, verbs},
};
use std::{cell::RefCell, net::SocketAddr, rc::Rc};
use thiserror;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    core::{PortForwarder, commands::ListResourcePortsCommand},
    kubernetes::{
        Kind, NAMESPACES, Namespace, ResourceRef, client::KubernetesClient, kinds::KindItem, resources::PODS, utils::get_resource,
    },
    ui::{views::PortForwardItem, widgets::FooterMessage},
};

use super::{
    BgDiscovery, BgExecutor, BgObserver, BgObserverError, History, SyntaxData, TaskResult,
    commands::{Command, DeleteResourcesCommand, GetResourceYamlCommand, SaveHistoryCommand},
};

pub type SharedBgWorker = Rc<RefCell<BgWorker>>;

/// Possible errors from [`BgWorkerError`].
#[derive(thiserror::Error, Debug)]
pub enum BgWorkerError {
    /// There is no kubernetes client to use.
    #[error("kubernetes client is not provided")]
    NoKubernetesClient,

    /// The background observer returned an error.
    #[error("background observer error")]
    BgObserverError(#[from] BgObserverError),
}

/// Keeps together all application background workers.
pub struct BgWorker {
    pub namespaces: BgObserver,
    pub resources: BgObserver,
    discovery: BgDiscovery,
    executor: BgExecutor,
    forwarder: PortForwarder,
    client: Option<KubernetesClient>,
    list: Option<Vec<(ApiResource, ApiCapabilities)>>,
}

impl BgWorker {
    /// Creates new [`BgWorker`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
            namespaces: BgObserver::new(footer_tx.clone()),
            resources: BgObserver::new(footer_tx.clone()),
            discovery: BgDiscovery::new(footer_tx.clone()),
            executor: BgExecutor::default(),
            forwarder: PortForwarder::new(footer_tx),
            client: None,
            list: None,
        }
    }

    /// Starts (or restarts) all background tasks that application requires to work.
    pub fn start(
        &mut self,
        client: KubernetesClient,
        initial_discovery_list: Vec<(ApiResource, ApiCapabilities)>,
        resource: ResourceRef,
    ) -> Result<Scope, BgWorkerError> {
        self.list = Some(initial_discovery_list);
        self.discovery.start(&client);

        let namespaces = Kind::from(NAMESPACES);
        let discovery = get_resource(self.list.as_ref(), &namespaces);
        self.namespaces
            .start(&client, ResourceRef::new(namespaces, Namespace::default()), discovery)?;

        let discovery = get_resource(self.list.as_ref(), &resource.kind);
        let scope = self.resources.start(&client, resource, discovery)?;

        self.client = Some(client);

        Ok(scope)
    }

    /// Restarts (if needed) the resources observer to change observed resource and namespace.
    pub fn restart(&mut self, resource: ResourceRef) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), &resource.kind);
            Ok(self.resources.restart(client, resource, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Restarts (if needed) the resources observer to change observed resource kind.
    pub fn restart_new_kind(&mut self, kind: Kind, last_namespace: Namespace) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), &kind);
            Ok(self.resources.restart_new_kind(client, kind, last_namespace, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Restarts (if needed) the resources observer to change observed namespace.
    pub fn restart_new_namespace(&mut self, resource_namespace: Namespace) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let mut discovery = get_resource(self.list.as_ref(), self.resources.get_resource_kind());
            if discovery.is_none() {
                discovery = get_resource(self.list.as_ref(), &PODS.into());
            }
            Ok(self.resources.restart_new_namespace(client, resource_namespace, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Restarts (if needed) the resources observer to show pod containers.
    pub fn restart_containers(&mut self, pod_name: String, pod_namespace: Namespace) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), &PODS.into());
            Ok(self
                .resources
                .restart_containers(client, pod_name, pod_namespace, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Stops all background tasks except the executor one.
    pub fn stop(&mut self) {
        self.namespaces.stop();
        self.resources.stop();
        self.discovery.stop();
        self.forwarder.stop_all();
    }

    /// Stops all background tasks running in the application.
    pub fn stop_all(&mut self) {
        self.namespaces.stop();
        self.resources.stop();
        self.executor.stop_all();
        self.discovery.stop();
        self.forwarder.stop_all();
    }

    /// Cancels all background tasks running in the application.
    pub fn cancel_all(&mut self) {
        self.namespaces.cancel();
        self.resources.cancel();
        self.executor.cancel_all();
        self.discovery.cancel();
        self.forwarder.stop_all();
    }

    /// Returns [`KubernetesClient`].
    pub fn kubernetes_client(&self) -> Option<&KubernetesClient> {
        self.client.as_ref()
    }

    /// Returns list of discovered kubernetes kinds.
    pub fn get_kinds_list(&self) -> Option<Vec<KindItem>> {
        self.list.as_ref().map(|discovery| {
            discovery
                .iter()
                .filter(|(_, cap)| cap.supports_operation(verbs::LIST))
                .map(|(ar, _)| KindItem::new(ar.group.clone(), ar.plural.clone(), ar.version.clone()))
                .collect::<Vec<KindItem>>()
        })
    }

    /// Returns list of port forward list items.
    pub fn get_port_forwards_list(&self) -> Vec<PortForwardItem> {
        self.forwarder.tasks().iter().map(PortForwardItem::from).collect()
    }

    /// Returns `true` if there was a change in the port forwards list since the last check.
    pub fn is_port_forward_list_changed(&mut self) -> bool {
        let mut list_changed = false;
        while self.forwarder.try_next().is_some() {
            list_changed = true;
        }

        list_changed
    }

    /// Checks and updates discovered resources list, returns `true` if discovery was updated.
    pub fn update_discovery_list(&mut self) -> bool {
        let discovery = self.discovery.try_next();
        if discovery.is_some() {
            self.list = discovery;
            true
        } else {
            false
        }
    }

    /// Sends the provided command to the background executor.
    pub fn run_command(&mut self, command: Command) -> String {
        self.executor.run_task(command)
    }

    /// Cancels command with the specified ID.
    pub fn cancel_command(&mut self, command_id: Option<&str>) {
        if let Some(id) = command_id {
            self.executor.cancel_task(id);
        }
    }

    /// Returns first waiting command result from the background executor.
    pub fn check_command_result(&mut self) -> Option<Box<TaskResult>> {
        self.executor.try_next()
    }

    /// Returns all waiting command results from the background executor.
    pub fn get_all_waiting_results(&mut self) -> Vec<Box<TaskResult>> {
        let mut commands = Vec::new();
        while let Some(command) = self.check_command_result() {
            commands.push(command);
        }
        commands
    }

    /// Returns `true` if there are connection problems.
    pub fn has_errors(&self) -> bool {
        self.resources.has_error() || self.namespaces.has_error() || self.discovery.has_error()
    }

    /// Saves the provided app history to a file.
    pub fn save_history(&mut self, history: History) {
        self.executor
            .run_task(Command::SaveHistory(Box::new(SaveHistoryCommand::new(history))));
    }

    /// Sends [`DeleteResourcesCommand`] to the background executor with provided resource names.
    pub fn delete_resources(&mut self, resources: Vec<String>, namespace: Namespace, kind: &Kind) {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), kind);
            let command = DeleteResourcesCommand::new(resources, namespace, discovery, client.get_client());
            self.executor.run_task(Command::DeleteResource(Box::new(command)));
        }
    }

    /// Sends [`ListResourcePortsCommand`] to the background executor.
    pub fn list_resource_ports(&mut self, resource: ResourceRef) {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), &resource.kind);
            let command = ListResourcePortsCommand::new(resource, discovery, client.get_client());
            self.executor.run_task(Command::ListResourcePorts(Box::new(command)));
        }
    }

    /// Sends [`GetResourceYamlCommand`] to the background executor.
    pub fn get_yaml(
        &mut self,
        name: String,
        namespace: Namespace,
        kind: &Kind,
        syntax: SyntaxData,
        decode: bool,
    ) -> Option<String> {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), kind);
            let command = if decode {
                GetResourceYamlCommand::decode(name, namespace, kind.clone(), discovery, client.get_client(), syntax)
            } else {
                GetResourceYamlCommand::new(name, namespace, kind.clone(), discovery, client.get_client(), syntax)
            };
            Some(self.executor.run_task(Command::GetYaml(Box::new(command))))
        } else {
            None
        }
    }

    /// Starts port forwarding for the specified resource, port and address.
    pub fn port_forward(&mut self, resource: ResourceRef, port: u16, address: SocketAddr) {
        if let Some(client) = &self.client {
            let _ = self.forwarder.start(client, resource, port, address);
        }
    }
}

impl Drop for BgWorker {
    fn drop(&mut self) {
        self.cancel_all();
    }
}
