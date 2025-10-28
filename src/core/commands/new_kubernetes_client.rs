use b4n_kube::{Kind, NAMESPACES, Namespace, PODS};
use kube::{Discovery, api::ListParams};
use thiserror;

use crate::{
    core::{DiscoveryList, discovery::convert_to_vector},
    kubernetes::{
        client::{ClientOptions, KubernetesClient},
        utils::get_resource,
    },
};

use super::CommandResult;

/// Possible errors when creating kubernetes client.
#[derive(thiserror::Error, Debug)]
pub enum KubernetesClientError {
    /// Kubernetes client creation error.
    #[error(transparent)]
    Client(#[from] crate::kubernetes::client::ClientError),

    /// Discovery run error.
    #[error("discovery run error")]
    DiscoveryFailure,

    /// Cannot get namespaces from the kubernetes cluster.
    #[error("cannot get namespaces from the kubernetes cluster")]
    NamespaceFetchFailure,
}

/// Result for the [`NewKubernetesClientCommand`].
pub struct KubernetesClientResult {
    pub client: KubernetesClient,
    pub kind: Kind,
    pub namespace: Namespace,
    pub discovery: DiscoveryList,
}

/// Command that creates new kubernetes client.
pub struct NewKubernetesClientCommand {
    pub kube_config_path: Option<String>,
    pub context: String,
    pub kind: Kind,
    pub namespace: Namespace,
    pub allow_insecure: bool,
}

impl NewKubernetesClientCommand {
    /// Creates new [`NewKubernetesClientCommand`] instance.
    pub fn new(
        kube_config_path: Option<String>,
        context: String,
        kind: Kind,
        namespace: Namespace,
        allow_insecure: bool,
    ) -> Self {
        Self {
            kube_config_path,
            context,
            kind,
            namespace,
            allow_insecure,
        }
    }

    /// Creates new kubernetes client and returns it.
    pub async fn execute(self) -> Option<CommandResult> {
        let client = KubernetesClient::new(
            self.kube_config_path.as_deref(),
            Some(&self.context),
            ClientOptions {
                fallback_to_default: false,
                allow_insecure: self.allow_insecure,
            },
        )
        .await;
        let client = match client {
            Ok(client) => client,
            Err(err) => return Some(CommandResult::KubernetesClient(Err(err.into()))),
        };
        let Ok(discovery) = Discovery::new(client.get_client()).run().await else {
            return Some(CommandResult::KubernetesClient(Err(KubernetesClientError::DiscoveryFailure)));
        };
        let discovery = convert_to_vector(&discovery);
        let kind = if get_resource(Some(&discovery), &self.kind).is_some() {
            self.kind
        } else {
            PODS.into()
        };
        let Some(namespaces) = get_resource(Some(&discovery), &NAMESPACES.into()) else {
            return Some(CommandResult::KubernetesClient(Err(
                KubernetesClientError::NamespaceFetchFailure,
            )));
        };
        let namespaces = client.get_api(&namespaces.0, &namespaces.1, None, true);
        let Ok(namespaces) = namespaces.list(&ListParams::default()).await else {
            return Some(CommandResult::KubernetesClient(Err(
                KubernetesClientError::NamespaceFetchFailure,
            )));
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
