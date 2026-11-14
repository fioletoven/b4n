use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, IconKind, NotificationSink};
use b4n_kube::{DrainOptions, wait_for_deletion};
use k8s_openapi::api::apps::v1::DaemonSet;
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::api::{DeleteParams, EvictParams, ListParams, Preconditions};
use kube::{Api, Client, ResourceExt};
use std::convert::identity;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};

use crate::commands::CommandResult;
use crate::commands::cordon_node::cordon_node;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const MIRROR_POD_ANNOTATION: &str = "kubernetes.io/config.mirror";

pub struct DrainNodeCommand {
    client: Client,
    name: String,
    options: DrainOptions,
    sink: NotificationSink,
}

impl DrainNodeCommand {
    pub fn new(client: Client, name: String, options: DrainOptions, sink: NotificationSink) -> Self {
        Self {
            client,
            name,
            options,
            sink,
        }
    }

    pub async fn execute(self) -> Option<CommandResult> {
        let result = self.run().await;
        self.clear_progress();

        match result {
            Ok(()) => {
                let message = format!("Node '{}' drained", self.name);
                tracing::info!("{message}");
                self.sink.show_info(message, DEFAULT_MESSAGE_DURATION);
            },
            Err(error) => {
                let message = format!("Cannot drain node '{}': {error}", self.name);
                tracing::error!("{message}");
                self.sink.show_error(message, DEFAULT_ERROR_DURATION);
            },
        }

        None
    }

    async fn run(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("node name is not provided".into());
        }

        let message = format!("Draining node '{}'", self.name);
        tracing::info!("{message}");
        self.sink.show_info(message, DEFAULT_MESSAGE_DURATION);

        match self.options.timeout_seconds {
            0 => self.drain().await,
            secs => timeout(Duration::from_secs(secs.into()), self.drain())
                .await
                .map_err(|_| "timed out - some pods may still be running".to_string())?,
        }
    }

    async fn drain(&self) -> Result<(), String> {
        cordon_node(self.client.clone(), &self.name, true)
            .await
            .map_err(|error| format!("cordon failed: {error}"))?;

        let params = ListParams::default().fields(&format!("spec.nodeName={}", self.name));
        let pods = Api::<Pod>::all(self.client.clone())
            .list(&params)
            .await
            .map_err(|error| format!("cannot list pods: {error}"))?
            .items;

        let mut targets = Vec::new();
        let mut errors = Vec::new();
        for pod in pods {
            match self.should_drain_pod(&pod).await {
                Ok(true) => targets.push(pod),
                Ok(false) => {},
                Err(error) => errors.push(error),
            }
        }
        into_result(errors)?;

        let total = targets.len();
        self.show_progress(0, total);

        let mut tasks = JoinSet::new();
        for pod in targets {
            tasks.spawn(drain_pod(self.client.clone(), pod, self.options.clone(), self.sink.clone()));
        }

        let mut done = 0;
        let mut errors = Vec::new();
        while let Some(result) = tasks.join_next().await {
            done += 1;
            self.show_progress(done, total);

            if let Err(error) = result
                .map_err(|error| format!("pod drain task failed: {error}"))
                .and_then(identity)
            {
                errors.push(error);
            }
        }
        into_result(errors)
    }

    async fn should_drain_pod(&self, pod: &Pod) -> Result<bool, String> {
        let id = pod_id(pod);
        let options = &self.options;

        if pod.annotations().contains_key(MIRROR_POD_ANNOTATION) {
            return Ok(false);
        }

        if pod.uid().is_none_or(|uid| uid.is_empty()) {
            return Err(format!("{id} has no UID"));
        }

        if is_finished(pod) {
            return Ok(true);
        }

        match controller_of(pod) {
            None if !options.force => return Err(format!("{id} has no controller")),
            Some(owner) if owner.kind == "DaemonSet" && owner.api_version == "apps/v1" => {
                if self
                    .daemon_set_exists(pod, owner)
                    .await
                    .map_err(|e| format!("cannot check DaemonSet for {id}: {e}"))?
                {
                    return if options.ignore_daemonsets {
                        Ok(false)
                    } else {
                        Err(format!("{id} is managed by a DaemonSet"))
                    };
                }
                if !options.force {
                    return Err(format!("{id} has no existing DaemonSet"));
                }
            },
            _ => {},
        }

        if !options.delete_empty_dir_data && has_empty_dir(pod) {
            return Err(format!("{id} uses emptyDir data"));
        }

        Ok(true)
    }

    async fn daemon_set_exists(&self, pod: &Pod, owner: &OwnerReference) -> kube::Result<bool> {
        let api: Api<DaemonSet> = Api::namespaced(self.client.clone(), &pod.namespace().unwrap_or_default());
        Ok(api
            .get_metadata_opt(&owner.name)
            .await?
            .is_some_and(|ds| ds.metadata.uid.as_deref() == Some(owner.uid.as_str())))
    }

    fn progress_id(&self) -> String {
        format!("drain:{}", self.name)
    }

    fn show_progress(&self, done: usize, total: usize) {
        self.sink.set_text(
            self.progress_id(),
            Some(format!("Draining: {done}/{total}")),
            IconKind::Default,
        );
    }

    fn clear_progress(&self) {
        self.sink.set_text(self.progress_id(), None::<String>, IconKind::Default);
    }
}

fn into_result(errors: Vec<String>) -> Result<(), String> {
    if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
}

fn pod_id(pod: &Pod) -> String {
    format!("{}/{}", pod.namespace().unwrap_or_default(), pod.name_any())
}

fn is_finished(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|status| status.phase.as_deref())
        .is_some_and(|phase| matches!(phase, "Succeeded" | "Failed"))
}

fn controller_of(pod: &Pod) -> Option<&OwnerReference> {
    pod.owner_references().iter().find(|owner| owner.controller == Some(true))
}

fn has_empty_dir(pod: &Pod) -> bool {
    pod.spec
        .as_ref()
        .and_then(|spec| spec.volumes.as_ref())
        .is_some_and(|volumes| volumes.iter().any(|volume| volume.empty_dir.is_some()))
}

async fn drain_pod(client: Client, pod: Pod, options: DrainOptions, sink: NotificationSink) -> Result<(), String> {
    let id = pod_id(&pod);
    let name = pod.name_any();
    let uid = pod.uid().unwrap_or_default();
    let api: Api<Pod> = Api::namespaced(client, &pod.namespace().unwrap_or_default());

    if pod.metadata.deletion_timestamp.is_none() {
        let requested = remove_pod(&api, &name, &uid, &options)
            .await
            .map_err(|error| format!("cannot remove {id}: {error}"))?;
        if !requested {
            return Ok(()); // already gone
        }
    }

    wait_for_deletion(&api, &name, &uid)
        .await
        .map_err(|error| format!("cannot wait for {id} to terminate: {error}"))?;

    let message = format!("Drained pod {id}");
    tracing::info!("{message}");
    sink.show_info(message, DEFAULT_MESSAGE_DURATION);

    Ok(())
}

/// Returns `Ok(true)` when removal was requested, `Ok(false)` when the pod was already gone.
async fn remove_pod(api: &Api<Pod>, name: &str, uid: &str, options: &DrainOptions) -> kube::Result<bool> {
    let params = DeleteParams {
        grace_period_seconds: options.grace_period_seconds,
        preconditions: Some(Preconditions {
            uid: Some(uid.to_owned()),
            resource_version: None,
        }),
        ..Default::default()
    };

    loop {
        let result = if options.disable_eviction {
            api.delete(name, &params).await.map(drop)
        } else {
            let evict = EvictParams {
                delete_options: Some(params.clone()),
                ..Default::default()
            };
            api.evict(name, &evict).await.map(drop)
        };

        match result {
            Ok(()) => return Ok(true),
            // Not found or conflict.
            Err(kube::Error::Api(error)) if matches!(error.code, 404 | 409) => {
                let same_pod_exists = api
                    .get_metadata_opt(name)
                    .await?
                    .is_some_and(|pod| pod.metadata.uid.as_deref() == Some(uid));
                return if same_pod_exists {
                    Err(kube::Error::Api(error))
                } else {
                    Ok(false)
                };
            },
            // Eviction blocked by a PodDisruptionBudget, retry.
            Err(kube::Error::Api(error)) if error.code == 429 && !options.disable_eviction => {
                sleep(POLL_INTERVAL).await;
            },
            Err(error) => return Err(error),
        }
    }
}
