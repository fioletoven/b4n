use b4n_config::APP_NAME;
use b4n_kube::{Namespace, ResourceRef};
use k8s_openapi::api::core::v1::{EphemeralContainer, Pod, SecurityContext};
use k8s_openapi::serde_json::json;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};

use crate::commands::CommandResult;

/// Possible errors from injecting container to the pod.
#[derive(thiserror::Error, Debug)]
pub enum InjectContainerError {
    /// Unable to inject container to the pod.
    #[error("unable to inject '{container_name}' to pod '{pod_name}': {source}")]
    KubeError {
        pod_name: String,
        container_name: String,
        #[source]
        source: kube::Error,
    },
}

/// Ephemeral container security profile.
pub enum SecurityProfile {
    Privileged,
    RunAsUser(i64),
    ReadOnly,
}

/// Configuration for the ephemeral container to create.
pub struct EphemeralContainerConfig {
    pub name: String,
    pub image: String,
    pub target_container: Option<String>,
    pub command: String,
    pub security_context: Option<SecurityProfile>,
}

/// Command that injects an ephemeral container to the specified pod.
pub struct InjectContainerCommand {
    name: String,
    namespace: Namespace,
    client: Client,
    config: EphemeralContainerConfig,
}

impl InjectContainerCommand {
    /// Creates new [`InjectContainerCommand`] instance.
    pub fn new(name: String, namespace: Namespace, client: Client, config: EphemeralContainerConfig) -> Self {
        Self {
            name,
            namespace,
            client,
            config,
        }
    }

    /// Injects ephemeral container to the specified pod.
    pub async fn execute(self) -> Option<CommandResult> {
        let api: Api<Pod> = Api::namespaced(self.client, self.namespace.as_str());
        Some(CommandResult::InjectedContainer(
            inject_container(api, self.name, self.config).await,
        ))
    }
}

async fn inject_container(
    api: Api<Pod>,
    name: String,
    config: EphemeralContainerConfig,
) -> Result<ResourceRef, InjectContainerError> {
    let pod = api.get(&name).await.map_err(|e| InjectContainerError::KubeError {
        pod_name: name.clone(),
        container_name: config.name.clone(),
        source: e,
    })?;

    let mut ephemeral_containers = pod
        .spec
        .as_ref()
        .and_then(|s| s.ephemeral_containers.clone())
        .unwrap_or_default();

    ephemeral_containers.push(build_ephemeral_container(&config));

    api.patch_subresource(
        "ephemeralcontainers",
        &name,
        &PatchParams::apply(APP_NAME),
        &Patch::Strategic(json!({
            "spec": {
                "ephemeralContainers": ephemeral_containers
            }
        })),
    )
    .await
    .map_err(|e| InjectContainerError::KubeError {
        pod_name: name.clone(),
        container_name: config.name.clone(),
        source: e,
    })?;

    Ok(ResourceRef::container(name, api.namespace().into(), config.name))
}

fn build_ephemeral_container(config: &EphemeralContainerConfig) -> EphemeralContainer {
    let command = if config.command.is_empty() {
        None
    } else {
        shlex::split(&config.command)
    };

    EphemeralContainer {
        name: config.name.clone(),
        image: Some(config.image.clone()),
        image_pull_policy: Some("Always".to_string()),
        command,
        stdin: Some(true),
        tty: Some(true),
        target_container_name: config.target_container.clone(),
        security_context: config.security_context.as_ref().map(|sc| SecurityContext {
            privileged: matches!(sc, SecurityProfile::Privileged).then_some(true),
            run_as_user: if let SecurityProfile::RunAsUser(uid) = sc {
                Some(*uid)
            } else {
                None
            },
            read_only_root_filesystem: matches!(sc, SecurityProfile::ReadOnly).then_some(true),
            ..Default::default()
        }),
        ..Default::default()
    }
}
