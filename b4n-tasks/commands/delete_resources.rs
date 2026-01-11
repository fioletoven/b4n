use b4n_common::{DEFAULT_ERROR_DURATION, NotificationSink};
use b4n_kube::{Namespace, PropagationPolicy};
use k8s_openapi::serde_json::json;
use kube::api::{ApiResource, DeleteParams, DynamicObject, Patch, PatchParams, Preconditions};
use kube::discovery::{ApiCapabilities, Scope, verbs};
use kube::{Api, Client};
use tokio::task::JoinSet;

use crate::commands::CommandResult;

/// Holds additional [`DeleteResourcesCommand`] options.
pub struct DeleteResourcesOptions {
    pub propagation_policy: PropagationPolicy,
    pub terminate_immediately: bool,
    pub detach_finalizers: bool,
}

/// Command that deletes all named resources for provided namespace and discovery.
pub struct DeleteResourcesCommand {
    pub resources: Vec<(String, String)>,
    pub namespace: Namespace,
    pub discovery: Option<(ApiResource, ApiCapabilities)>,
    pub client: Client,
    options: DeleteResourcesOptions,
    footer_tx: NotificationSink,
}

impl DeleteResourcesCommand {
    /// Creates new [`DeleteResourcesCommand`] instance.
    pub fn new(
        resources: Vec<(String, String)>,
        namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        delete_options: DeleteResourcesOptions,
        footer_tx: NotificationSink,
    ) -> Self {
        Self {
            resources,
            namespace,
            discovery,
            client,
            options: delete_options,
            footer_tx,
        }
    }

    /// Deletes all resources using provided client.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let (client, info, delete_params) = self.prepare_context()?;
        tracing::info!(
            "About to delete the following resources: {} ({})",
            self.resources
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            info
        );

        let mut set = JoinSet::new();

        for (name, uid) in self.resources {
            let info = info.clone();
            let client = client.clone();
            let mut delete_params = delete_params.clone();
            let detach_finalizers = self.options.detach_finalizers;
            let footer_tx = self.footer_tx.clone();

            set.spawn(async move {
                if detach_finalizers {
                    let patch = json!({ "metadata": { "finalizers": null } });

                    if let Err(err) = client.patch(&name, &PatchParams::default(), &Patch::Merge(&patch)).await {
                        let msg = format!("Cannot detach finalizers from {name} ({info}): {err}");
                        tracing::error!(msg);
                        footer_tx.show_error(msg, 0);

                        return;
                    }

                    let msg = format!("Detached finalizers from {name} ({info})");
                    tracing::info!(msg);
                    footer_tx.show_info(msg, 0);
                }

                if !uid.is_empty() {
                    delete_params.preconditions = Some(Preconditions {
                        resource_version: None,
                        uid: Some(uid),
                    });
                }

                if let Err(err) = client.delete(&name, &delete_params).await {
                    let msg = format!("Cannot delete resource {name} ({info}): {err}");
                    tracing::error!(msg);
                    footer_tx.show_error(msg, 0);
                } else {
                    let msg = format!("Deleted resource {name} ({info})");
                    tracing::info!(msg);
                    footer_tx.show_info(msg, 0);
                }
            });
        }

        while let Some(res) = set.join_next().await {
            if let Err(err) = res {
                let msg = format!("Delete task failed to complete: {err}");
                tracing::error!(msg);
                self.footer_tx.show_error(msg, DEFAULT_ERROR_DURATION);
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
            info = format!("kind: {}", discovery.0.plural);
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

        let delete_params = if self.options.terminate_immediately {
            DeleteParams {
                grace_period_seconds: Some(0),
                propagation_policy: self.options.propagation_policy.into(),
                ..Default::default()
            }
        } else {
            DeleteParams::default()
        };

        Some((client, info, delete_params))
    }
}
