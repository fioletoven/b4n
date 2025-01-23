use kube::{api::ApiResource, discovery::ApiCapabilities, Discovery};
use thiserror;

use crate::{
    app::discovery::convert_to_vector,
    kubernetes::{client::KubernetesClient, Namespace},
};

use super::CommandResult;

/// Possible errors when creating kubernetes client.
#[derive(thiserror::Error, Debug)]
pub enum KubernetesClientError {
    /// Kubernetes client creation error
    #[error("kubernetes client creation error")]
    ClientError,

    /// Discovery run error
    #[error("discovery run error")]
    DiscoveryError,
}

/// Result for the [`NewKubernetesClientCommand`].
pub struct KubernetesClientResult {
    pub client: KubernetesClient,
    pub kind: String,
    pub namespace: Namespace,
    pub discovery: Vec<(ApiResource, ApiCapabilities)>,
}

/// Command that creates new kubernetes client.
pub struct NewKubernetesClientCommand {
    pub context: String,
    pub kind: String,
    pub namespace: Namespace,
}

impl NewKubernetesClientCommand {
    /// Creates new [`NewKubernetesClientCommand`] instance.
    pub fn new(context: String, kind: String, namespace: Namespace) -> Self {
        Self {
            context,
            kind,
            namespace,
        }
    }

    /// Creates new kubernetes client and returns it.
    pub async fn execute(&self) -> Option<CommandResult> {
        if let Ok(client) = KubernetesClient::new(Some(&self.context), false).await {
            if let Ok(discovery) = Discovery::new(client.get_client()).run().await {
                return Some(CommandResult::KubernetesClient(Ok(KubernetesClientResult {
                    client,
                    kind: self.kind.clone(),
                    namespace: self.namespace.clone(),
                    discovery: convert_to_vector(&discovery),
                })));
            } else {
                return Some(CommandResult::KubernetesClient(Err(KubernetesClientError::DiscoveryError)));
            }
        } else {
            return Some(CommandResult::KubernetesClient(Err(KubernetesClientError::ClientError)));
        }
    }
}
