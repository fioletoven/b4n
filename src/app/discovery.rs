use backoff::backoff::Backoff;
use kube::{Discovery, api::ApiResource, discovery::ApiCapabilities};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::{kubernetes::client::KubernetesClient, ui::widgets::FooterMessage};

use super::utils::{build_default_backoff, wait_for_task};

const DISCOVERY_INTERVAL: u64 = 6_000;

/// Background Kubernetes API discovery.
pub struct BgDiscovery {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<Vec<(ApiResource, ApiCapabilities)>>,
    context_rx: UnboundedReceiver<Vec<(ApiResource, ApiCapabilities)>>,
    footer_tx: UnboundedSender<FooterMessage>,
    has_error: Arc<AtomicBool>,
}

impl BgDiscovery {
    /// Creates new [`BgDiscovery`] instance.
    pub fn new(footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
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

        let task = tokio::spawn(async move {
            let mut backoff = build_default_backoff();
            let mut next_interval = Duration::from_millis(DISCOVERY_INTERVAL);

            let mut maybe_discovery = Some(Discovery::new(_client.clone()));
            while !_cancellation_token.is_cancelled() {
                if let Some(discovery) = maybe_discovery.take() {
                    tokio::select! {
                        _ = _cancellation_token.cancelled() => (),
                        result = discovery.run() => match result {
                            Ok(new_discovery) => {
                                _context_tx.send(convert_to_vector(&new_discovery)).unwrap();
                                _has_error.store(false, Ordering::Relaxed);
                                maybe_discovery = Some(new_discovery);
                                next_interval = Duration::from_millis(DISCOVERY_INTERVAL);
                            }
                            Err(error) => {
                                let msg = format!("Discovery error: {}", error);
                                warn!("{}", msg);
                                _footer_tx.send(FooterMessage::error(msg, 0)).unwrap();
                                if !_has_error.swap(true, Ordering::Relaxed) || backoff.start_time.elapsed().as_secs() > 120 {
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
                    _ = _cancellation_token.cancelled() => (),
                    _ = sleep(next_interval) => (),
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
        wait_for_task(self.task.take(), "discovery");
    }

    /// Tries to get next discovery result.
    pub fn try_next(&mut self) -> Option<Vec<(ApiResource, ApiCapabilities)>> {
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
pub fn convert_to_vector(discovery: &Discovery) -> Vec<(ApiResource, ApiCapabilities)> {
    discovery.groups().flat_map(|g| g.resources_by_stability()).collect()
}
