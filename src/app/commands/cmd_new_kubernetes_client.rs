use kube::{api::ApiResource, discovery::ApiCapabilities, Discovery};

use crate::{
    app::discovery::convert_to_vector,
    kubernetes::{client::KubernetesClient, Namespace},
};

use super::ExecutorResult;

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
    pub async fn execute(&self) -> Option<ExecutorResult> {
        if let Ok(client) = KubernetesClient::new(Some(&self.context), false).await {
            if let Ok(discovery) = Discovery::new(client.get_client()).run().await {
                return Some(ExecutorResult::KubernetesClient(KubernetesClientResult {
                    client,
                    kind: self.kind.clone(),
                    namespace: self.namespace.clone(),
                    discovery: convert_to_vector(&discovery),
                }));
            }
        }

        None
    }
}
