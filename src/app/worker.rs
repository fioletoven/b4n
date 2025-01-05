use anyhow::Result;
use kube::{
    api::ApiResource,
    discovery::{verbs, ApiCapabilities, Scope},
};
use std::time::Duration;
use tokio::time::sleep;

use crate::kubernetes::{client::KubernetesClient, resources::Kind, NAMESPACES};

use super::{
    commands::{BgExecutor, Command, DeleteResourcesCommand, SaveConfigCommand},
    BgDiscovery, BgObserver, BgObserverError, Config,
};

/// Keeps together all application background workers
pub struct BgWorker {
    pub namespaces: BgObserver,
    pub resources: BgObserver,
    discovery: BgDiscovery,
    executor: BgExecutor,
    client: KubernetesClient,
    list: Option<Vec<(ApiResource, ApiCapabilities)>>,
}

impl BgWorker {
    /// Creates new [`BgWorker`] instance
    pub fn new(client: KubernetesClient) -> Self {
        Self {
            namespaces: BgObserver::default(),
            resources: BgObserver::default(),
            discovery: BgDiscovery::default(),
            executor: BgExecutor::default(),
            client,
            list: None,
        }
    }

    /// Starts all background observers that application requires to work.  
    /// Consumes the first event from [`BgDiscovery`] as it waits to populate list of resources.
    pub async fn start(&mut self, resource_name: String, resource_namespace: Option<String>) -> Result<Scope, BgObserverError> {
        self.start_discovery().await;

        self.executor.start(&self.client);

        let discovery = self.get_resource(NAMESPACES);
        self.namespaces.start(&self.client, NAMESPACES.to_owned(), None, discovery)?;

        let discovery = self.get_resource(&resource_name);
        self.resources
            .start(&self.client, resource_name, resource_namespace, discovery)
    }

    /// Restarts (if needed) the resources observer to change observed resource kind and namespace
    pub fn restart(&mut self, resource_name: String, resource_namespace: Option<String>) -> Result<Scope, BgObserverError> {
        let discovery = self.get_resource(&resource_name);
        self.resources
            .restart(&self.client, resource_name, resource_namespace, discovery)
    }

    /// Restarts (if needed) the resources observer to change observed resource kind
    pub fn restart_new_kind(&mut self, resource_name: String) -> Result<Scope, BgObserverError> {
        let discovery = self.get_resource(&resource_name);
        self.resources.restart_new_kind(&self.client, resource_name, discovery)
    }

    /// Restarts (if needed) the resources observer to change observed namespace
    pub fn restart_new_namespace(&mut self, resource_namespace: Option<String>) -> Result<Scope, BgObserverError> {
        let discovery = self.get_resource(self.resources.get_resource_name());
        self.resources
            .restart_new_namespace(&self.client, resource_namespace, discovery)
    }

    /// Stops all background tasks running in the application
    pub fn stop(&mut self) {
        self.namespaces.stop();
        self.resources.stop();
        self.executor.stop();
        self.discovery.stop();
    }

    /// Cancels all background tasks running in the application
    pub fn cancel(&mut self) {
        self.namespaces.cancel();
        self.resources.cancel();
        self.executor.cancel();
        self.discovery.cancel();
    }

    /// Returns list of discovered kubernetes kinds
    pub fn get_kinds_list(&self) -> Option<Vec<Kind>> {
        self.list.as_ref().map(|discovery| {
            discovery
                .iter()
                .filter(|(_, cap)| cap.supports_operation(verbs::LIST))
                .map(|(ar, _)| Kind::new(ar.group.to_owned(), ar.plural.to_owned(), ar.version.to_owned()))
                .collect::<Vec<Kind>>()
        })
    }

    /// Checks and updates discovered resources list, returns `true` if discovery was updated
    pub fn update_discovery_list(&mut self) -> bool {
        let discovery = self.discovery.try_next();
        if discovery.is_some() {
            self.list = discovery;
            true
        } else {
            false
        }
    }

    /// Saves the provided configuration to a file
    pub fn save_configuration(&self, config: Config) {
        self.executor
            .run_command(Command::SaveConfiguration(SaveConfigCommand::new(config)));
    }

    /// Sends [`DeleteResourcesCommand`] to the background executor with provided resource names.  
    pub fn delete_resources(&self, resources: Vec<String>, namespace: Option<String>, kind: &str) {
        let discovery = self.get_resource(kind);
        let command = DeleteResourcesCommand::new(resources, namespace, discovery);
        self.executor.run_command(Command::DeleteResource(command));
    }

    /// Returns `true` if there are connection problems
    pub fn has_errors(&self) -> bool {
        self.resources.has_error() || self.namespaces.has_error() || self.discovery.has_error()
    }

    /// Returns kube context name
    pub fn context(&self) -> &str {
        self.client.context()
    }

    /// Returns kubernetes API version
    pub fn k8s_version(&self) -> &str {
        self.client.k8s_version()
    }

    /// Starts kubernetes resources discovery and waits for the first result
    async fn start_discovery(&mut self) {
        self.discovery.start(&self.client);

        let mut discovery = self.discovery.try_next();
        while discovery.is_none() {
            sleep(Duration::from_millis(50)).await;
            discovery = self.discovery.try_next();
        }

        self.list = discovery;
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

    /// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name and group
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

    /// Gets first matching [`ApiResource`] and [`ApiCapabilities`] for the resource name ignoring group
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
        self.cancel();
    }
}
