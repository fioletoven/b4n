use kube::{
    api::{ApiResource, DeleteParams},
    discovery::{verbs, ApiCapabilities, Scope},
    Client,
};
use tracing::error;

use crate::kubernetes;

use super::ExecutorResult;

/// Command that deletes all named resources for provided namespace and discovery.
pub struct DeleteResourcesCommand {
    pub names: Vec<String>,
    pub namespace: Option<String>,
    pub discovery: Option<(ApiResource, ApiCapabilities)>,
}

impl DeleteResourcesCommand {
    /// Creates new [`DeleteResourcesCommand`] instance.
    pub fn new(names: Vec<String>, namespace: Option<String>, discovery: Option<(ApiResource, ApiCapabilities)>) -> Self {
        Self {
            names,
            namespace,
            discovery,
        }
    }

    /// Deletes all resources using provided client.
    pub async fn execute(&mut self, client: &Client) -> Option<ExecutorResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::DELETE) {
            return None;
        }

        let namespace = if discovery.1.scope == Scope::Cluster {
            None
        } else {
            self.namespace.as_deref()
        };
        let client = kubernetes::client::get_dynamic_api(
            discovery.0.clone(),
            discovery.1.clone(),
            client.clone(),
            namespace,
            namespace.is_none(),
        );

        for name in &self.names {
            let deleted = client.delete(name, &DeleteParams::default()).await;
            if let Err(error) = deleted {
                error!("Cannot delete resource {}: {}", name, error);
            }
        }

        None
    }
}
