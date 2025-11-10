use b4n_kube::Namespace;
use futures::future::join_all;
use k8s_openapi::serde_json::json;
use kube::Client;
use kube::api::{ApiResource, DeleteParams, Patch, PatchParams};
use kube::discovery::{ApiCapabilities, Scope, verbs};

use crate::commands::CommandResult;

/// Command that deletes all named resources for provided namespace and discovery.
pub struct DeleteResourcesCommand {
    pub names: Vec<String>,
    pub namespace: Namespace,
    pub discovery: Option<(ApiResource, ApiCapabilities)>,
    pub client: Client,
    terminate_immediately: bool,
    detach_finalizers: bool,
}

impl DeleteResourcesCommand {
    /// Creates new [`DeleteResourcesCommand`] instance.
    pub fn new(
        names: Vec<String>,
        namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        terminate_immediately: bool,
        detach_finalizers: bool,
    ) -> Self {
        Self {
            names,
            namespace,
            discovery,
            client,
            terminate_immediately,
            detach_finalizers,
        }
    }

    /// Deletes all resources using provided client.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::DELETE) {
            return None;
        }

        let namespace;
        let info;
        if discovery.1.scope == Scope::Cluster {
            namespace = None;
            info = discovery.0.plural.clone();
        } else {
            namespace = self.namespace.as_option();
            info = format!("{}, ns: {}", discovery.0.plural, namespace.unwrap_or("n/a"));
        }

        let client = b4n_kube::client::get_dynamic_api(&discovery.0, &discovery.1, self.client, namespace, namespace.is_none());

        let delete_params = if self.terminate_immediately {
            DeleteParams {
                grace_period_seconds: Some(0),
                ..Default::default()
            }
        } else {
            DeleteParams::default()
        };

        let tasks = self.names.into_iter().map(|name| {
            let info = info.clone();
            let client = client.clone();
            let delete_params = delete_params.clone();
            let detach_finalizers = self.detach_finalizers;

            tokio::spawn(async move {
                if detach_finalizers {
                    let patch = json!({ "metadata": { "finalizers": null } });

                    if let Err(err) = client.patch(&name, &PatchParams::default(), &Patch::Merge(&patch)).await {
                        tracing::error!("Cannot detach finalizers from {} ({}): {}", name, info, err);
                    } else {
                        tracing::info!("Detached finalizers from {} ({})", name, info);
                    }
                }

                if let Err(err) = client.delete(&name, &delete_params).await {
                    tracing::error!("Cannot delete resource {} ({}): {}", name, info, err);
                } else {
                    tracing::info!("Deleted resource {} ({})", name, info);
                }
            })
        });

        join_all(tasks).await;

        None
    }
}
