use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::PathBuf,
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
use tracing::info;

use crate::app::utils::wait_for_task;

use super::Config;

/// Observes for changes in configuration file.
pub struct ConfigWatcher {
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    config_tx: UnboundedSender<Config>,
    config_rx: UnboundedReceiver<Config>,
    skip_next: Arc<AtomicBool>,
}

impl ConfigWatcher {
    /// Creates new [`ConfigWatcher`] instance.
    pub fn new(config_to_watch: PathBuf) -> Self {
        let (config_tx, config_rx) = mpsc::unbounded_channel();
        Self {
            path: config_to_watch,
            watcher: None,
            task: None,
            cancellation_token: None,
            config_tx,
            config_rx,
            skip_next: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Runs a background task to observe configuration changes.
    pub fn start(&mut self) -> Result<()> {
        let (mut _tx, mut _rx) = mpsc::channel(10);
        let mut watcher = RecommendedWatcher::new(
            move |result| {
                _tx.blocking_send(result).expect("Failed to send configuration change event");
            },
            notify::Config::default(),
        )?;

        watcher.watch(&self.path, RecursiveMode::NonRecursive)?;
        self.watcher = Some(watcher);

        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _config_tx = self.config_tx.clone();
        let _skip_next = Arc::clone(&self.skip_next);

        let task = tokio::spawn(async move {
            while !_cancellation_token.is_cancelled() {
                sleep(Duration::from_millis(500)).await;

                let mut configuration_modified = false;
                while let Ok(res) = _rx.try_recv() {
                    info!("got {:?}", res);
                    if let Ok(res) = res {
                        if let EventKind::Modify(_) = res.kind {
                            configuration_modified = true
                        }
                    }
                }

                if configuration_modified && !_skip_next.swap(false, Ordering::Relaxed) {
                    info!("loading configuration because of change...");
                    if let Ok(config) = Config::load().await {
                        _config_tx.send(config).unwrap();
                    }
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);

        Ok(())
    }

    /// Cancels [`ConfigWatcher`] task.
    pub fn cancel(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`ConfigWatcher`] task and waits until it is finished.
    pub fn stop(&mut self) {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            wait_for_task(self.task.take(), "configuration watcher");
        }
    }

    /// Sets watcher to skip the next modification event.
    pub fn skip_next(&mut self) {
        self.skip_next.store(true, Ordering::Relaxed);
    }

    /// Tries to get a new configuration if it has been reloaded due to a file modification.
    pub fn try_next(&mut self) -> Option<Config> {
        self.config_rx.try_recv().ok()
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.cancel();
        if let Some(watcher) = &mut self.watcher {
            let _ = watcher.unwatch(&self.path);
        }
    }
}
