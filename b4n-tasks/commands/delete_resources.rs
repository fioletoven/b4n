use b4n_kube::Namespace;
use k8s_openapi::serde_json::json;
use kube::api::{ApiResource, DeleteParams, DynamicObject, Patch, PatchParams, Preconditions};
use kube::discovery::{ApiCapabilities, Scope, verbs};
use kube::{Api, Client};
use tokio::task::JoinSet;

use crate::commands::CommandResult;

/// Command that deletes all named resources for provided namespace and discovery.
pub struct DeleteResourcesCommand {
    pub names: Vec<String>,
    pub uids: Vec<String>,
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
        uids: Vec<String>,
        namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        terminate_immediately: bool,
        detach_finalizers: bool,
    ) -> Self {
        Self {
            names,
            uids,
            namespace,
            discovery,
            client,
            terminate_immediately,
            detach_finalizers,
        }
    }

    /// Deletes all resources using provided client.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let (client, info, delete_params) = self.prepare_context()?;
        tracing::info!(
            "About to delete the following resources: {} ({})",
            self.names.join(", "),
            info
        );

        let mut set = JoinSet::new();

        for (name, uid) in self.names.into_iter().zip(self.uids.into_iter()) {
            let info = info.clone();
            let client = client.clone();
            let mut delete_params = delete_params.clone();
            let detach_finalizers = self.detach_finalizers;

            set.spawn(async move {
                if detach_finalizers {
                    let patch = json!({ "metadata": { "finalizers": null } });

                    if let Err(err) = client.patch(&name, &PatchParams::default(), &Patch::Merge(&patch)).await {
                        tracing::error!("Cannot detach finalizers from {} ({}): {}", name, info, err);
                        return;
                    }

                    tracing::info!("Detached finalizers from {} ({})", name, info);
                }

                if !uid.is_empty() {
                    delete_params.preconditions = Some(Preconditions {
                        resource_version: None,
                        uid: Some(uid),
                    });
                }

                if let Err(err) = client.delete(&name, &delete_params).await {
                    tracing::error!("Cannot delete resource {} ({}): {}", name, info, err);
                } else {
                    tracing::info!("Deleted resource {} ({})", name, info);
                }
            });
        }

        while let Some(res) = set.join_next().await {
            if let Err(err) = res {
                tracing::error!("Delete task failed to complete: {}", err);
            }
        }

        None
    }

    fn prepare_context(&mut self) -> Option<(Api<DynamicObject>, String, DeleteParams)> {
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
            info = format!("kind: {}, ns: {}", discovery.0.plural, namespace.unwrap_or("n/a"));
        }

        let client = b4n_kube::client::get_dynamic_api(
            &discovery.0,
            &discovery.1,
            self.client.clone(),
            namespace,
            namespace.is_none(),
        );

        let delete_params = if self.terminate_immediately {
            DeleteParams {
                grace_period_seconds: Some(0),
                ..Default::default()
            }
        } else {
            DeleteParams::default()
        };

        Some((client, info, delete_params))
    }
}
