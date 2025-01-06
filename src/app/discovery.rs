use kube::{api::ApiResource, discovery::ApiCapabilities, Discovery};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
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

use crate::kubernetes::client::KubernetesClient;

use super::utils::wait_for_task;

/// Background Kubernetes API discovery
pub struct BgDiscovery {
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    context_tx: UnboundedSender<Vec<(ApiResource, ApiCapabilities)>>,
    context_rx: UnboundedReceiver<Vec<(ApiResource, ApiCapabilities)>>,
    has_error: Arc<AtomicBool>,
}

impl Default for BgDiscovery {
    fn default() -> Self {
        let (context_tx, context_rx) = mpsc::unbounded_channel();
        Self {
            task: None,
            cancellation_token: None,
            context_tx,
            context_rx,
            has_error: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl BgDiscovery {
    /// Starts new [`BgDiscovery`] task
    pub fn start(&mut self, client: &KubernetesClient) {
        let cancellation_token = CancellationToken::new();

        let _cancellation_token = cancellation_token.clone();
        let _context_tx = self.context_tx.clone();

        self.has_error.store(false, Ordering::Relaxed);
        let _has_error = Arc::clone(&self.has_error);

        let _client = client.get_client();

        let task = tokio::spawn(async move {
            let mut discovery = Discovery::new(_client.clone());

            while !_cancellation_token.is_cancelled() {
                match discovery.run().await {
                    Ok(new_discovery) => {
                        discovery = new_discovery;
                        _has_error.store(false, Ordering::Relaxed);
                        _context_tx.send(convert_to_vector(&discovery)).unwrap();
                    }
                    Err(error) => {
                        warn!("Cannot run discovery: {:?}", error);
                        _has_error.store(true, Ordering::Relaxed);
                        discovery = Discovery::new(_client.clone());
                    }
                };

                tokio::select! {
                    _ = _cancellation_token.cancelled() => (),
                    _ = sleep(Duration::from_millis(6_000)) => (),
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);
    }

    /// Cancels [`BgDiscovery`] task
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`BgDiscovery`] task and waits until it is finished
    pub fn stop(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            wait_for_task(self.task.take(), "discovery");
        }
    }

    /// Tries to get next discovery result
    pub fn try_next(&mut self) -> Option<Vec<(ApiResource, ApiCapabilities)>> {
        self.context_rx.try_recv().ok()
    }

    /// Returns `true` if discovery is in an error state
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
    }
}

impl Drop for BgDiscovery {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Converts [`Discovery`] to vector of [`ApiResource`] and [`ApiCapabilities`]
#[inline]
fn convert_to_vector(discovery: &Discovery) -> Vec<(ApiResource, ApiCapabilities)> {
    discovery.groups().flat_map(|g| g.resources_by_stability()).collect()
}
