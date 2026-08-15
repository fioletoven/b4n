use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self, Receiver, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::ConfigError;

/// Configurations that can be saved to and load from a file.
pub trait Persistable<T> {
    /// Returns the default configuration path.
    fn default_path() -> PathBuf;

    /// Loads configuration from the default file.
    fn load(path: &Path) -> impl Future<Output = Result<T, ConfigError>> + Send;

    /// Saves configuration to the default file.
    fn save(&self, path: &Path) -> impl Future<Output = Result<(), ConfigError>> + Send;
}

/// Observes for changes in the configuration file.
pub struct ConfigWatcher<T: Persistable<T> + Default + Send + 'static> {
    path: PathBuf,
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    config_tx: UnboundedSender<Result<T, ConfigError>>,
    config_rx: UnboundedReceiver<Result<T, ConfigError>>,
    force_reload: Arc<AtomicBool>,
    skip_next: Arc<AtomicBool>,
}

impl<T: Persistable<T> + Default + Send + 'static> ConfigWatcher<T> {
    /// Creates new [`ConfigWatcher`] instance.
    pub fn new(runtime: Handle, path: PathBuf) -> Self {
        let (config_tx, config_rx) = mpsc::unbounded_channel();
        Self {
            path,
            runtime,
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
        let (tx, rx) = mpsc::channel(10);
        let watcher = RecommendedWatcher::new(
            move |result| {
                if let Err(error) = tx.blocking_send(result) {
                    tracing::warn!("Failed to send configuration change event: {}", error);
                }
            },
            notify::Config::default(),
        )?;

        let cancellation_token = CancellationToken::new();
        self.skip_next.store(false, Ordering::Relaxed);

        self.task = Some(self.runtime.spawn(Self::watch_task(
            watcher,
            rx,
            self.path.clone(),
            self.config_tx.clone(),
            Arc::clone(&self.force_reload),
            Arc::clone(&self.skip_next),
            cancellation_token.clone(),
        )));
        self.cancellation_token = Some(cancellation_token);

        Ok(())
    }

    /// Changes the observed configuration file to the specified one and restarts the [`ConfigWatcher`].\
    /// **Note** that this will force a reload of the observed file.
    pub fn change_file(&mut self, path: PathBuf) -> Result<()> {
        self.stop();
        self.path = path;
        self.skip_next.store(false, Ordering::Relaxed);
        self.force_reload.store(true, Ordering::Relaxed);
        self.start()
    }

    /// Cancels [`ConfigWatcher`] task.
    pub fn cancel(&mut self) {
        if let Some(token) = self.cancellation_token.take() {
            token.cancel();
        }
    }

    /// Cancels [`ConfigWatcher`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        b4n_common::tasks::wait_for_task(self.task.take(), "configuration watcher");
    }

    /// Sets watcher to skip the next modification event.
    pub fn skip_next(&mut self) {
        self.skip_next.store(true, Ordering::Relaxed);
    }

    /// Tries to get a new configuration if it has been reloaded due to a file modification.
    pub fn try_next(&mut self) -> Option<Result<T, ConfigError>> {
        self.config_rx.try_recv().ok()
    }

    async fn watch_task(
        mut watcher: RecommendedWatcher,
        mut watcher_rx: Receiver<notify::Result<notify::Event>>,
        path: PathBuf,
        config_tx: UnboundedSender<Result<T, ConfigError>>,
        force_reload: Arc<AtomicBool>,
        skip_next: Arc<AtomicBool>,
        cancellation_token: CancellationToken,
    ) {
        let use_default = T::default_path() == path;
        let mut watcher_had_errors = false;

        while !cancellation_token.is_cancelled() {
            if force_reload.swap(false, Ordering::Relaxed) && !skip_next.swap(false, Ordering::Relaxed) {
                Self::load_and_send_file(&config_tx, &path, use_default).await;
            }

            if watcher.watch(&path, RecursiveMode::NonRecursive).is_err() {
                if !watcher_had_errors {
                    watcher_had_errors = true;
                    let _ = config_tx.send(Err(ConfigError::NotFound));
                }
                tokio::select! {
                    () = sleep(Duration::from_secs(15)) => {}
                    () = cancellation_token.cancelled() => {}
                }
                continue;
            }

            if watcher_had_errors {
                watcher_had_errors = false;
                force_reload.store(false, Ordering::Relaxed);
                if !skip_next.swap(false, Ordering::Relaxed) {
                    Self::load_and_send_file(&config_tx, &path, use_default).await;
                }
            }

            'w: while !cancellation_token.is_cancelled() {
                sleep(Duration::from_millis(500)).await;

                let mut file_modified = false;
                let mut file_removed = false;

                while let Ok(event) = watcher_rx.try_recv() {
                    if let Ok(res) = event {
                        file_removed |= matches!(res.kind, EventKind::Remove(_));
                        file_modified |= matches!(res.kind, EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_));
                    } else {
                        let _ = watcher.unwatch(&path);
                        break 'w;
                    }
                }

                if file_removed && tokio::fs::metadata(&path).await.is_err() {
                    let _ = watcher.unwatch(&path);
                    break 'w;
                }

                if file_modified && !skip_next.swap(false, Ordering::Relaxed) {
                    force_reload.store(false, Ordering::Relaxed);
                    Self::load_and_send_file(&config_tx, &path, use_default).await;
                }
            }
        }
    }

    async fn load_and_send_file(tx: &UnboundedSender<Result<T, ConfigError>>, path: &Path, use_default: bool) {
        let result = match T::load(path).await {
            Ok(file) => Ok(file),
            Err(error) if use_default && !matches!(error, ConfigError::DeserializationError(_)) => Ok(T::default()),
            Err(error) => Err(error),
        };
        let _ = tx.send(result);
    }
}

impl<T: Persistable<T> + Default + Send + 'static> Drop for ConfigWatcher<T> {
    fn drop(&mut self) {
        self.cancel();
    }
}
