use b4n_kube::Namespace;
use kube::{
    Client,
    api::{ApiResource, DeleteParams},
    discovery::{ApiCapabilities, Scope, verbs},
};
use tracing::error;

use super::CommandResult;

/// Command that deletes all named resources for provided namespace and discovery.
pub struct DeleteResourcesCommand {
    pub names: Vec<String>,
    pub namespace: Namespace,
    pub discovery: Option<(ApiResource, ApiCapabilities)>,
    pub client: Client,
    force: bool,
}

impl DeleteResourcesCommand {
    /// Creates new [`DeleteResourcesCommand`] instance.
    pub fn new(
        names: Vec<String>,
        namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        force: bool,
    ) -> Self {
        Self {
            names,
            namespace,
            discovery,
            client,
            force,
        }
    }

    /// Deletes all resources using provided client.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::DELETE) {
            return None;
        }

        let namespace = if discovery.1.scope == Scope::Cluster {
            None
        } else {
            self.namespace.as_option()
        };
        let client = b4n_kube::client::get_dynamic_api(&discovery.0, &discovery.1, self.client, namespace, namespace.is_none());

        let params = if self.force {
            DeleteParams {
                grace_period_seconds: Some(0),
                ..Default::default()
            }
        } else {
            DeleteParams::default()
        };

        for name in &self.names {
            let deleted = client.delete(name, &params).await;
            if let Err(error) = deleted {
                error!("Cannot delete resource {}: {}", name, error);
            }
        }

        None
    }
}
