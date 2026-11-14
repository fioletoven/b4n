use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use kube::api::{WatchEvent, WatchParams};
use std::pin::pin;
use std::time::Duration;
use tokio::time::sleep;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const WATCH_TIMEOUT_SECONDS: u32 = 30;
const MAX_WATCH_FAILURES: u32 = 3;

/// Waits until the pod with the given UID is gone (deleted or replaced by a pod with a different UID).
pub async fn wait_for_deletion(api: &Api<Pod>, name: &str, uid: &str) -> kube::Result<()> {
    let params = WatchParams::default()
        .fields(&format!("metadata.name={name}"))
        .timeout(WATCH_TIMEOUT_SECONDS);

    let mut watch_failures = 0;
    loop {
        let Some(pod) = api.get_metadata_opt(name).await? else {
            return Ok(());
        };
        if pod.metadata.uid.as_deref() != Some(uid) {
            return Ok(());
        }

        if watch_failures < MAX_WATCH_FAILURES
            && let Some(version) = pod.metadata.resource_version.as_deref()
        {
            match watch_until_gone(api, &params, version, uid).await {
                WatchOutcome::Gone => return Ok(()),
                WatchOutcome::Resync => watch_failures = 0,
                WatchOutcome::Failed => watch_failures += 1,
            }
        }

        sleep(POLL_INTERVAL).await;
    }
}

enum WatchOutcome {
    Gone,
    Resync,
    Failed,
}

async fn watch_until_gone(api: &Api<Pod>, params: &WatchParams, version: &str, uid: &str) -> WatchOutcome {
    let Ok(stream) = api.watch_metadata(params, version).await else {
        return WatchOutcome::Failed;
    };

    let mut stream = pin!(stream);
    loop {
        match stream.try_next().await {
            Ok(Some(WatchEvent::Deleted(pod))) if pod.metadata.uid.as_deref() == Some(uid) => {
                return WatchOutcome::Gone;
            },
            // A different pod took over the name, ours is gone.
            Ok(Some(WatchEvent::Added(pod) | WatchEvent::Modified(pod))) if pod.metadata.uid.as_deref() != Some(uid) => {
                return WatchOutcome::Gone;
            },
            Ok(Some(WatchEvent::Error(_))) | Err(_) => return WatchOutcome::Failed,
            Ok(None) => return WatchOutcome::Resync,
            Ok(Some(_)) => {},
        }
    }
}
