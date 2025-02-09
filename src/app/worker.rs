use anyhow::Result;
use kube::{
    api::ApiResource,
    discovery::{verbs, ApiCapabilities, Scope},
};
use thiserror;

use crate::kubernetes::{client::KubernetesClient, resources::Kind, utils::get_resource, Namespace, NAMESPACES};

use super::{
    commands::{Command, DeleteResourcesCommand, GetYamlCommand, SaveConfigurationCommand},
    BgDiscovery, BgExecutor, BgObserver, BgObserverError, Config, TaskResult,
};

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
#[derive(Default)]
pub struct BgWorker {
    pub namespaces: BgObserver,
    pub resources: BgObserver,
    discovery: BgDiscovery,
    executor: BgExecutor,
    client: Option<KubernetesClient>,
    list: Option<Vec<(ApiResource, ApiCapabilities)>>,
}

impl BgWorker {
    /// Starts (or restarts) all background tasks that application requires to work.
    pub fn start(
        &mut self,
        client: KubernetesClient,
        initial_discovery_list: Vec<(ApiResource, ApiCapabilities)>,
        resource_name: String,
        resource_namespace: Namespace,
    ) -> Result<Scope, BgWorkerError> {
        self.list = Some(initial_discovery_list);
        self.discovery.start(&client);

        let discovery = get_resource(self.list.as_ref(), NAMESPACES);
        self.namespaces
            .start(&client, NAMESPACES.to_owned(), Namespace::default(), discovery)?;

        let discovery = get_resource(self.list.as_ref(), &resource_name);
        let scope = self.resources.start(&client, resource_name, resource_namespace, discovery)?;

        self.client = Some(client);

        Ok(scope)
    }

    /// Restarts (if needed) the resources observer to change observed resource kind and namespace.
    pub fn restart(&mut self, resource_name: String, resource_namespace: Namespace) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), &resource_name);
            Ok(self.resources.restart(client, resource_name, resource_namespace, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Restarts (if needed) the resources observer to change observed resource kind.
    pub fn restart_new_kind(&mut self, kind: String, last_namespace: Namespace) -> Result<Scope, BgWorkerError> {
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
            let discovery = get_resource(self.list.as_ref(), self.resources.get_resource_name());
            Ok(self.resources.restart_new_namespace(client, resource_namespace, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Stops all background tasks except the executor one.
    pub fn stop(&mut self) {
        self.namespaces.stop();
        self.resources.stop();
        self.discovery.stop();
    }

    /// Stops all background tasks running in the application.
    pub fn stop_all(&mut self) {
        self.namespaces.stop();
        self.resources.stop();
        self.executor.stop_all();
        self.discovery.stop();
    }

    /// Cancels all background tasks running in the application.
    pub fn cancel_all(&mut self) {
        self.namespaces.cancel();
        self.resources.cancel();
        self.executor.cancel_all();
        self.discovery.cancel();
    }

    /// Returns list of discovered kubernetes kinds.
    pub fn get_kinds_list(&self) -> Option<Vec<Kind>> {
        self.list.as_ref().map(|discovery| {
            discovery
                .iter()
                .filter(|(_, cap)| cap.supports_operation(verbs::LIST))
                .map(|(ar, _)| Kind::new(ar.group.to_owned(), ar.plural.to_owned(), ar.version.to_owned()))
                .collect::<Vec<Kind>>()
        })
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

    /// Saves the provided configuration to a file.
    pub fn save_configuration(&mut self, config: Config) {
        self.executor
            .run_task(Command::SaveConfiguration(Box::new(SaveConfigurationCommand::new(config))));
    }

    /// Sends [`DeleteResourcesCommand`] to the background executor with provided resource names.  
    pub fn delete_resources(&mut self, resources: Vec<String>, namespace: Namespace, kind: &str) {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), kind);
            let command = DeleteResourcesCommand::new(resources, namespace, discovery, client.get_client());
            self.executor.run_task(Command::DeleteResource(Box::new(command)));
        }
    }

    /// Sends [`GetYamlCommand`] to the background executor.
    pub fn get_yaml(&mut self, name: String, namespace: Namespace, kind: &str) {
        if let Some(client) = &self.client {
            let discovery = get_resource(self.list.as_ref(), kind);
            let command = GetYamlCommand::new(name, namespace, discovery, client.get_client());
            self.executor.run_task(Command::GetYaml(Box::new(command)));
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
    pub fn check_command_result(&mut self) -> Option<TaskResult> {
        self.executor.try_next()
    }

    /// Returns `true` if there are connection problems.
    pub fn has_errors(&self) -> bool {
        self.resources.has_error() || self.namespaces.has_error() || self.discovery.has_error()
    }
}

impl Drop for BgWorker {
    fn drop(&mut self) {
        self.cancel_all();
    }
}
