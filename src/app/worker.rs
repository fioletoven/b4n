use anyhow::Result;
use kube::{
    api::ApiResource,
    discovery::{verbs, ApiCapabilities, Scope},
};
use thiserror;

use crate::kubernetes::{client::KubernetesClient, resources::Kind, Namespace, NAMESPACES};

use super::{
    commands::{BgExecutor, DeleteResourcesCommand, ExecutorCommand, ExecutorResult, SaveConfigurationCommand},
    BgDiscovery, BgObserver, BgObserverError, Config,
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

        let discovery = self.get_resource(NAMESPACES);
        self.namespaces
            .start(&client, NAMESPACES.to_owned(), Namespace::default(), discovery)?;

        let discovery = self.get_resource(&resource_name);
        let scope = self.resources.start(&client, resource_name, resource_namespace, discovery)?;

        self.client = Some(client);

        Ok(scope)
    }

    /// Restarts (if needed) the resources observer to change observed resource kind and namespace.
    pub fn restart(&mut self, resource_name: String, resource_namespace: Namespace) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let discovery = self.get_resource(&resource_name);
            Ok(self.resources.restart(client, resource_name, resource_namespace, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Restarts (if needed) the resources observer to change observed resource kind.
    pub fn restart_new_kind(&mut self, kind: String, last_namespace: Namespace) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let discovery = self.get_resource(&kind);
            Ok(self.resources.restart_new_kind(client, kind, last_namespace, discovery)?)
        } else {
            Err(BgWorkerError::NoKubernetesClient)
        }
    }

    /// Restarts (if needed) the resources observer to change observed namespace.
    pub fn restart_new_namespace(&mut self, resource_namespace: Namespace) -> Result<Scope, BgWorkerError> {
        if let Some(client) = &self.client {
            let discovery = self.get_resource(self.resources.get_resource_name());
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
            .run_task(ExecutorCommand::SaveConfiguration(SaveConfigurationCommand::new(config)));
    }

    /// Sends [`DeleteResourcesCommand`] to the background executor with provided resource names.  
    pub fn delete_resources(&mut self, resources: Vec<String>, namespace: Namespace, kind: &str) {
        let discovery = self.get_resource(kind);
        if let Some(client) = &self.client {
            let command = DeleteResourcesCommand::new(resources, namespace, discovery, client.get_client());
            self.executor.run_task(ExecutorCommand::DeleteResource(command));
        }
    }

    /// Sends the provided command to the background executor.
    pub fn run_command(&mut self, command: ExecutorCommand) {
        self.executor.run_task(command);
    }

    /// Returns first waiting command result from the background executor.
    pub fn check_command_result(&mut self) -> Option<ExecutorResult> {
        self.executor.try_next()
    }

    /// Returns `true` if there are connection problems.
    pub fn has_errors(&self) -> bool {
        self.resources.has_error() || self.namespaces.has_error() || self.discovery.has_error()
    }

    /// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name.  
    /// Name value can be in the form `name.group`.
    fn get_resource(&self, name: &str) -> Option<(ApiResource, ApiCapabilities)> {
        if name.contains('.') {
            let mut split = name.splitn(2, '.');
            self.get_resource_with_group(split.next().unwrap(), split.next().unwrap())
        } else {
            self.get_resource_no_group(name)
        }
    }

    /// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name and group.
    fn get_resource_with_group(&self, name: &str, group: &str) -> Option<(ApiResource, ApiCapabilities)> {
        if group.is_empty() {
            self.get_resource_no_group(name)
        } else {
            self.list.as_ref().and_then(|discovery| {
                discovery
                    .iter()
                    .find(|(ar, _)| {
                        group.eq_ignore_ascii_case(&ar.group)
                            && (name.eq_ignore_ascii_case(&ar.kind) || name.eq_ignore_ascii_case(&ar.plural))
                    })
                    .map(|(ar, cap)| (ar.clone(), cap.clone()))
            })
        }
    }

    /// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name ignoring group.
    fn get_resource_no_group(&self, name: &str) -> Option<(ApiResource, ApiCapabilities)> {
        self.list.as_ref().and_then(|discovery| {
            discovery
                .iter()
                .filter(|(ar, _)| name.eq_ignore_ascii_case(&ar.kind) || name.eq_ignore_ascii_case(&ar.plural))
                .min_by_key(|(ar, _)| &ar.group)
                .map(|(ar, cap)| (ar.clone(), cap.clone()))
        })
    }
}

impl Drop for BgWorker {
    fn drop(&mut self) {
        self.cancel_all();
    }
}
