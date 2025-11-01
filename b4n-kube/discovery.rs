use b4n_common::NotificationSink;
use kube::{Discovery, api::ApiResource, discovery::ApiCapabilities};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::client::KubernetesClient;

pub type DiscoveryList = Vec<(ApiResource, ApiCapabilities)>;

const DISCOVERY_INTERVAL: u64 = 6_000;

/// Background Kubernetes API discovery.
pub struct BgDiscovery {
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<DiscoveryList>,
    context_rx: UnboundedReceiver<DiscoveryList>,
    footer_tx: NotificationSink,
    has_error: Arc<AtomicBool>,
}

impl BgDiscovery {
    /// Creates new [`BgDiscovery`] instance.
    pub fn new(runtime: Handle, footer_tx: NotificationSink) -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            runtime,
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
            footer_tx,
            has_error: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Starts new [`BgDiscovery`] task.
    pub fn start(&mut self, client: &KubernetesClient) {
        if self.cancellation_token.is_some() {
            self.stop();
        }

        let cancellation_token = CancellationToken::new();

        let _cancellation_token = cancellation_token.clone();
        let _context_tx = self.context_tx.clone();

        self.has_error.store(false, Ordering::Relaxed);
        let _has_error = Arc::clone(&self.has_error);
        let _footer_tx = self.footer_tx.clone();

        let _client = client.get_client();

        let task = self.runtime.spawn(async move {
            let mut backoff = b4n_common::ResettableBackoff::default();
            let mut next_interval = Duration::from_millis(DISCOVERY_INTERVAL);

            let mut maybe_discovery = Some(Discovery::new(_client.clone()));
            while !_cancellation_token.is_cancelled() {
                if let Some(discovery) = maybe_discovery.take() {
                    tokio::select! {
                        () = _cancellation_token.cancelled() => (),
                        result = discovery.run() => match result {
                            Ok(new_discovery) => {
                                _context_tx.send(convert_to_vector(&new_discovery)).unwrap();
                                _has_error.store(false, Ordering::Relaxed);
                                maybe_discovery = Some(new_discovery);
                                next_interval = Duration::from_millis(DISCOVERY_INTERVAL);
                            }
                            Err(error) => {
                                let msg = format!("Discovery error: {error}");
                                tracing::warn!("{}", msg);
                                _footer_tx.show_error(msg, 0);
                                if !_has_error.swap(true, Ordering::Relaxed) {
                                    backoff.reset();
                                }
                                maybe_discovery = Some(Discovery::new(_client.clone()));
                                next_interval = backoff.next_backoff().unwrap_or(Duration::from_millis(DISCOVERY_INTERVAL));
                            }
                        },
                    }
                }

                if maybe_discovery.is_none() {
                    break;
                }

                tokio::select! {
                    () = _cancellation_token.cancelled() => (),
                    () = sleep(next_interval) => (),
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
    }

    /// Cancels [`BgDiscovery`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            self.has_error.store(true, Ordering::Relaxed);
        }
    }

    /// Cancels [`BgDiscovery`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        b4n_common::tasks::wait_for_task(self.task.take(), "discovery");
    }

    /// Tries to get next discovery result.
    pub fn try_next(&mut self) -> Option<DiscoveryList> {
        self.context_rx.try_recv().ok()
    }

    /// Returns `true` if discovery is not running or is in an error state.
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
    }
}

impl Drop for BgDiscovery {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Converts [`Discovery`] to vector of [`ApiResource`] and [`ApiCapabilities`].
#[inline]
pub fn convert_to_vector(discovery: &Discovery) -> DiscoveryList {
    discovery
        .groups()
        .flat_map(|group| group.versions().flat_map(|version| group.versioned_resources(version)))
        .collect()
}
