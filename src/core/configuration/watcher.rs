use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::PathBuf,
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

use crate::core::utils::wait_for_task;

use super::Persistable;

/// Observes for changes in the configuration file.
pub struct ConfigWatcher<T: Persistable<T> + Send + 'static> {
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    config_tx: UnboundedSender<T>,
    config_rx: UnboundedReceiver<T>,
    force_reload: Arc<AtomicBool>,
    skip_next: Arc<AtomicBool>,
}

impl<T: Persistable<T> + Send + 'static> ConfigWatcher<T> {
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
            force_reload: Arc::new(AtomicBool::new(false)),
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
        let _path = self.path.clone();
        let _config_tx = self.config_tx.clone();
        let _force_reload = Arc::clone(&self.force_reload);
        let _skip_next = Arc::clone(&self.skip_next);
        self.skip_next.store(false, Ordering::Relaxed);

        let task = tokio::spawn(async move {
            while !_cancellation_token.is_cancelled() {
                sleep(Duration::from_millis(500)).await;

                let mut configuration_modified = false;
                while let Ok(res) = _rx.try_recv() {
                    if let Ok(res) = res {
                        if let EventKind::Modify(_) = res.kind {
                            configuration_modified = true;
                        }
                    }
                }

                if (configuration_modified && !_skip_next.swap(false, Ordering::Relaxed))
                    || _force_reload.swap(false, Ordering::Relaxed)
                {
                    if let Ok(config) = T::load(&_path).await {
                        _config_tx.send(config).unwrap();
                    }
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);

        Ok(())
    }

    /// Changes the observed configuration file to the specified one and restarts the [`ConfigWatcher`].\
    /// **Note** that this will force a reload of the observed file.
    pub fn change_file(&mut self, config_to_watch: PathBuf) -> Result<()> {
        self.stop();
        self.path = config_to_watch;
        self.skip_next.store(false, Ordering::Relaxed);
        self.force_reload.store(true, Ordering::Relaxed);
        self.start()
    }

    /// Cancels [`ConfigWatcher`] task.
    pub fn cancel(&mut self) {
        self.stop_watcher();
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`ConfigWatcher`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        wait_for_task(self.task.take(), "configuration watcher");
    }

    /// Sets watcher to skip the next modification event.
    pub fn skip_next(&mut self) {
        self.skip_next.store(true, Ordering::Relaxed);
    }

    /// Tries to get a new configuration if it has been reloaded due to a file modification.
    pub fn try_next(&mut self) -> Option<T> {
        self.config_rx.try_recv().ok()
    }

    fn stop_watcher(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            let _ = watcher.unwatch(&self.path);
        }
    }
}

impl<T: Persistable<T> + Send + 'static> Drop for ConfigWatcher<T> {
    fn drop(&mut self) {
        self.cancel();
    }
}
