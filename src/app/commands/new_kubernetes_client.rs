use kube::{Discovery, api::ApiResource, discovery::ApiCapabilities};
use thiserror;

use crate::{
    app::discovery::convert_to_vector,
    kubernetes::{Kind, NAMESPACES, Namespace, client::KubernetesClient, resources::PODS, utils::get_resource},
};

use super::CommandResult;

/// Possible errors when creating kubernetes client.
#[derive(thiserror::Error, Debug)]
pub enum KubernetesClientError {
    /// Kubernetes client creation error.
    #[error(transparent)]
    ClientError(#[from] crate::kubernetes::client::ClientError),

    /// Discovery run error.
    #[error("discovery run error")]
    DiscoveryError,

    /// Cannot get namespaces from the kubernetes cluster.
    #[error("cannot get namespaces from the kubernetes cluster")]
    NamespacesError,
}

/// Result for the [`NewKubernetesClientCommand`].
pub struct KubernetesClientResult {
    pub client: KubernetesClient,
    pub kind: Kind,
    pub namespace: Namespace,
    pub discovery: Vec<(ApiResource, ApiCapabilities)>,
}

/// Command that creates new kubernetes client.
pub struct NewKubernetesClientCommand {
    pub kube_config_path: Option<String>,
    pub context: String,
    pub kind: Kind,
    pub namespace: Namespace,
}

impl NewKubernetesClientCommand {
    /// Creates new [`NewKubernetesClientCommand`] instance.
    pub fn new(kube_config_path: Option<String>, context: String, kind: Kind, namespace: Namespace) -> Self {
        Self {
            kube_config_path,
            context,
            kind,
            namespace,
        }
    }

    /// Creates new kubernetes client and returns it.
    pub async fn execute(self) -> Option<CommandResult> {
        let client = match KubernetesClient::new(self.kube_config_path.as_deref(), Some(&self.context), false).await {
            Ok(client) => client,
            Err(err) => return Some(CommandResult::KubernetesClient(Err(err.into()))),
        };
        let Ok(discovery) = Discovery::new(client.get_client()).run().await else {
            return Some(CommandResult::KubernetesClient(Err(KubernetesClientError::DiscoveryError)));
        };
        let discovery = convert_to_vector(&discovery);
        let kind = if get_resource(Some(&discovery), &self.kind).is_some() {
            self.kind
        } else {
            PODS.into()
        };
        let Some(namespaces) = get_resource(Some(&discovery), &NAMESPACES.into()) else {
            return Some(CommandResult::KubernetesClient(Err(KubernetesClientError::NamespacesError)));
        };
        let namespaces = client.get_api(namespaces.0, namespaces.1, None, true);
        let Ok(namespaces) = namespaces.list(&Default::default()).await else {
            return Some(CommandResult::KubernetesClient(Err(KubernetesClientError::NamespacesError)));
        };
        let namespace = if namespaces.iter().any(|n| self.namespace.is_equal(n.metadata.name.as_deref())) {
            self.namespace
        } else {
            Namespace::default()
        };

        Some(CommandResult::KubernetesClient(Ok(KubernetesClientResult {
            client,
            kind,
            namespace,
            discovery,
        })))
    }
}
