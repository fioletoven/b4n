use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{APP_NAME, keys::KeyCombination};

/// Possible errors from plugins loading.
#[derive(thiserror::Error, Debug)]
pub enum PluginError {
    /// Cannot load plugins.
    #[error("cannot load plugins")]
    IoError(#[from] std::io::Error),

    /// Cannot deserialize plugin.
    #[error("cannot deserialize plugin")]
    DeserializationError(#[from] serde_saphyr::Error),
}

/// Holds particular plugin configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Plugin {
    #[serde(skip_deserializing, default = "random_uuid")]
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub shortcut: KeyCombination,
    pub command: String,
    pub args: Vec<String>,
    pub scopes: Vec<String>,
    pub confirm: bool,
    pub interactive: bool,
    pub highlighted: bool,
    pub selected: bool,
}

/// All discovered plugins.
#[derive(Debug, Default)]
pub struct Plugins(Vec<Plugin>);

impl Deref for Plugins {
    type Target = Vec<Plugin>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Plugins {
    /// Returns the default plugins path: `HOME/b4n/plugins/`.
    pub fn default_path() -> PathBuf {
        match std::env::home_dir() {
            Some(path) => path.join(format!(".{APP_NAME}")).join("plugins"),
            None => PathBuf::from("plugins"),
        }
    }

    /// Loads plugins from the specified directory.
    pub async fn from_directory(path: &Path) -> Result<Self, PluginError> {
        let mut plugins = Vec::new();
        let mut dir = fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if let Some(extension) = path.extension()
                && extension.eq_ignore_ascii_case("yaml")
            {
                let metadata = fs::metadata(&path).await?;
                if metadata.is_file() {
                    let mut file = File::open(path).await?;

                    let mut plugin_str = String::new();
                    file.read_to_string(&mut plugin_str).await?;

                    plugins.push(serde_saphyr::from_str::<Plugin>(&plugin_str)?);
                }
            }
        }

        Ok(Self(plugins))
    }
}

/// Observes for changes in the plugins directory.
pub struct PluginsWatcher {
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    runtime: Handle,
    task: Option<JoinHandle<()>>,
    cancellation_token: Option<CancellationToken>,
    plugins_tx: UnboundedSender<Result<Plugins, PluginError>>,
    plugins_rx: UnboundedReceiver<Result<Plugins, PluginError>>,
}

impl PluginsWatcher {
    /// Creates new [`PluginsWatcher`] instance.
    pub fn new(runtime: Handle, dir_to_watch: PathBuf) -> Self {
        let (plugins_tx, plugins_rx) = mpsc::unbounded_channel();
        Self {
            path: dir_to_watch,
            watcher: None,
            runtime,
            task: None,
            cancellation_token: None,
            plugins_tx,
            plugins_rx,
        }
    }

    /// Runs a background task to observe plugins changes.
    pub fn start(&mut self) -> Result<()> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let _path = self.path.clone();
        let _config_tx = self.plugins_tx.clone();

        let (tx, mut rx) = mpsc::channel(10);
        let mut watcher = RecommendedWatcher::new(
            move |result| {
                if let Err(error) = tx.blocking_send(result) {
                    tracing::warn!("Failed to send plugins change event: {}", error);
                }
            },
            notify::Config::default(),
        )?;

        let task = self.runtime.spawn(async move {
            while !_cancellation_token.is_cancelled() {
                if watcher.watch(&_path, RecursiveMode::NonRecursive).is_err() {
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }

                let _ = _config_tx.send(Plugins::from_directory(&_path).await);

                'w: while !_cancellation_token.is_cancelled() {
                    sleep(Duration::from_millis(500)).await;

                    let mut needs_reload = false;
                    let mut needs_check = false;

                    while let Ok(event) = rx.try_recv() {
                        match event {
                            Ok(res) => {
                                needs_check = matches!(res.kind, EventKind::Remove(_));
                                if matches!(res.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) {
                                    needs_reload = true;
                                }
                            },
                            Err(_) => {
                                let _ = watcher.unwatch(&_path);
                                break 'w;
                            },
                        }
                    }

                    if needs_check && fs::metadata(&_path).await.is_err() {
                        let _ = _config_tx.send(Ok(Plugins::default()));
                        let _ = watcher.unwatch(&_path);
                        break 'w;
                    }

                    if needs_reload {
                        let _ = _config_tx.send(Plugins::from_directory(&_path).await);
                    }
                }
            }
        });

        self.cancellation_token = Some(cancellation_token);
        self.task = Some(task);

        Ok(())
    }

    /// Cancels [`PluginsWatcher`] task.
    pub fn cancel(&mut self) {
        self.stop_watcher();
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
        }
    }

    /// Cancels [`PluginsWatcher`] task and waits until it is finished.
    pub fn stop(&mut self) {
        self.cancel();
        b4n_common::tasks::wait_for_task(self.task.take(), "plugins watcher");
    }

    /// Tries to get a new plugins instance if it has been reloaded due to a directory changes.
    pub fn try_next(&mut self) -> Option<Result<Plugins, PluginError>> {
        self.plugins_rx.try_recv().ok()
    }

    fn stop_watcher(&mut self) {
        if let Some(mut watcher) = self.watcher.take() {
            let _ = watcher.unwatch(&self.path);
        }
    }
}

impl Drop for PluginsWatcher {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn random_uuid() -> String {
    Uuid::new_v4()
        .hyphenated()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_owned()
}
