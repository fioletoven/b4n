use b4n_kube::{ContainerRef, Namespace, ResourceRef};
use k8s_openapi::api::core::v1::{Container, Pod, PodSpec, PodStatus, SecurityContext, Toleration};
use kube::api::{DeleteParams, PostParams};
use kube::{Api, Client};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use super::CommandResult;

const CONTAINER_NAME: &str = "shell";
const READY_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const ACTIVE_DEADLINE_SECONDS: i64 = 5400; // 1,5h
const FATAL_WAITING_REASONS: &[&str] = &[
    "InvalidImageName",
    "ImagePullBackOff",
    "ErrImageNeverPull",
    "CreateContainerConfigError",
    "CreateContainerError",
];

/// Errors that may occur while creating node shell pod.
#[derive(thiserror::Error, Debug)]
pub enum CreateNodeShellError {
    /// Unable to create node shell pod.
    #[error("unable to create node shell pod: {0}")]
    Kube(#[from] kube::Error),

    /// Created pod has no name.
    #[error("created node shell pod has no name")]
    MissingName,

    /// Pod did not get ready when expected (120s).
    #[error("node shell pod did not become ready within 120 seconds")]
    Timeout,

    /// Pod run error.
    #[error("node shell pod failed: {0}")]
    Failed(String),
}

/// Command that creates a shell pod for the specified kubernetes node.
pub struct CreateNodeShellCommand {
    client: Client,
    node: String,
    namespace: Namespace,
    image: String,
    cancellation: CancellationToken,
}

impl CreateNodeShellCommand {
    /// Creates new [`CreateNodeShellCommand`] instance.
    pub fn new(client: Client, node: String, namespace: Namespace, image: String, cancellation: CancellationToken) -> Self {
        Self {
            client,
            node,
            namespace,
            image,
            cancellation,
        }
    }

    /// Creates a shell pod for the specified kubernetes node.
    pub async fn execute(self) -> Option<CommandResult> {
        let api: Api<Pod> = Api::namespaced(self.client, self.namespace.as_str());
        let pod = match api.create(&PostParams::default(), &build_pod(self.node, self.image)).await {
            Ok(pod) => pod,
            Err(error) => return Some(CommandResult::CreatedNodeShell(Err(error.into()))),
        };
        let Some(name) = pod.metadata.name else {
            return Some(CommandResult::CreatedNodeShell(Err(CreateNodeShellError::MissingName)));
        };

        let pod_token = self.cancellation.child_token();

        let cleanup = tokio::spawn({
            let api = api.clone();
            let name = name.clone();
            let token = pod_token.clone();

            async move {
                token.cancelled().await;
                if let Err(error) = api.delete(&name, &DeleteParams::default()).await {
                    let not_found = matches!(&error, kube::Error::Api(status) if status.code == 404);
                    if !not_found {
                        tracing::warn!("Unable to delete node shell pod '{}': {}", name, error);
                    }
                }
            }
        });

        let result = tokio::select! {
            biased;
            () = pod_token.cancelled() => {
                let _ = cleanup.await;
                return None;
            },
            result = tokio::time::timeout(READY_TIMEOUT, wait_ready(&api, &name)) => {
                result.unwrap_or(Err(CreateNodeShellError::Timeout))
            }
        };

        if result.is_err() {
            pod_token.cancel();
            let _ = cleanup.await;
        }

        Some(CommandResult::CreatedNodeShell(result.map(|()| {
            ContainerRef::from(ResourceRef::container(name, self.namespace, CONTAINER_NAME.to_owned()))
        })))
    }
}

fn build_pod(node: String, image: String) -> Pod {
    Pod {
        metadata: kube::api::ObjectMeta {
            generate_name: Some("b4n-node-shell-".to_owned()),
            ..Default::default()
        },
        spec: Some(PodSpec {
            node_name: Some(node),
            host_pid: Some(true),
            host_network: Some(true),
            restart_policy: Some("Never".to_owned()),
            active_deadline_seconds: Some(ACTIVE_DEADLINE_SECONDS),
            termination_grace_period_seconds: Some(1),
            automount_service_account_token: Some(false),
            tolerations: Some(vec![Toleration {
                operator: Some("Exists".to_owned()),
                ..Default::default()
            }]),
            containers: vec![Container {
                name: CONTAINER_NAME.to_owned(),
                image: Some(image),
                command: Some(vec!["sleep".to_owned(), ACTIVE_DEADLINE_SECONDS.to_string()]),
                security_context: Some(SecurityContext {
                    privileged: Some(true),
                    run_as_user: Some(0),
                    run_as_non_root: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn wait_ready(api: &Api<Pod>, name: &str) -> Result<(), CreateNodeShellError> {
    loop {
        let pod = api.get(name).await?;

        if let Some(status) = &pod.status {
            if matches!(status.phase.as_deref(), Some("Failed" | "Succeeded")) {
                return Err(CreateNodeShellError::Failed(get_status_description(status)));
            }

            let shell = status
                .container_statuses
                .iter()
                .flatten()
                .find(|container| container.name == CONTAINER_NAME);

            if let Some(shell) = shell {
                if shell.ready {
                    return Ok(());
                }

                let waiting = shell.state.as_ref().and_then(|state| state.waiting.as_ref());
                if let Some(waiting) = waiting
                    && let Some(reason) = waiting.reason.as_deref()
                    && FATAL_WAITING_REASONS.contains(&reason)
                {
                    return Err(CreateNodeShellError::Failed(match waiting.message.as_deref() {
                        Some(message) if !message.is_empty() => format!("{reason}: {message}"),
                        _ => reason.to_owned(),
                    }));
                }
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

fn get_status_description(status: &PodStatus) -> String {
    let terminated = status
        .container_statuses
        .iter()
        .flatten()
        .filter(|container| container.name == CONTAINER_NAME)
        .find_map(|container| container.state.as_ref()?.terminated.as_ref());

    if let Some(terminated) = terminated {
        let mut text = format!(
            "container terminated: {} (exit code {})",
            terminated.reason.as_deref().unwrap_or("unknown reason"),
            terminated.exit_code
        );

        if let Some(message) = terminated.message.as_deref().filter(|message| !message.is_empty()) {
            text.push_str(": ");
            text.push_str(message);
        }

        return text;
    }

    status
        .message
        .clone()
        .filter(|message| !message.is_empty())
        .or_else(|| status.reason.clone())
        .unwrap_or_else(|| "unknown error".to_owned())
}
