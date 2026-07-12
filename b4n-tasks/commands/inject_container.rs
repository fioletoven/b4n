use b4n_config::APP_NAME;
use b4n_kube::{Namespace, ResourceRef};
use k8s_openapi::api::core::v1::{EphemeralContainer, Pod, SecurityContext};
use k8s_openapi::serde_json::json;
use kube::api::{Patch, PatchParams};
use kube::runtime::wait::{Condition, await_condition};
use kube::{Api, Client};
use std::time::Duration;

use crate::commands::CommandResult;

/// Possible errors from injecting container to the pod.
#[derive(thiserror::Error, Debug)]
pub enum InjectContainerError {
    /// Unable to inject container to the pod.
    #[error("unable to inject container to the pod")]
    KubeError(#[from] kube::Error),

    /// Error while waiting for container to be ready.
    #[error("error while waiting for container to be ready")]
    WaitError(#[from] kube::runtime::wait::Error),

    /// Waiting for container timed out.
    #[error("waiting for container timed out")]
    WaitTimeout(#[from] tokio::time::error::Elapsed),
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
    pub share_process_namespace: bool,
    pub security_context: Option<SecurityProfile>,
    pub wait_for_container: bool,
    pub wait_timeout: Option<u64>,
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
    let pod = api.get(&name).await?;

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
    .await?;

    if config.wait_for_container {
        wait_for_ephemeral_container(&api, &name, &config).await?;
    }

    Ok(ResourceRef::container(name, api.namespace().into(), config.name))
}

async fn wait_for_ephemeral_container(
    api: &Api<Pod>,
    pod_name: &str,
    config: &EphemeralContainerConfig,
) -> Result<Option<Pod>, InjectContainerError> {
    let condition = await_condition(api.clone(), pod_name, ephemeral_container_running(config.name.clone()));
    Ok(match config.wait_timeout {
        Some(secs) => tokio::time::timeout(Duration::from_secs(secs), condition).await??,
        None => condition.await?,
    })
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

fn ephemeral_container_running(container_name: String) -> impl Condition<Pod> {
    move |pod: Option<&Pod>| {
        pod.and_then(|p| p.status.as_ref())
            .and_then(|s| s.ephemeral_container_statuses.as_ref())
            .and_then(|statuses| statuses.iter().find(|s| s.name == container_name))
            .and_then(|s| s.state.as_ref())
            .and_then(|state| state.running.as_ref())
            .is_some()
    }
}
