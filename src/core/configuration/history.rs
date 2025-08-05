use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{core::ResourcesInfo, utils::calculate_hash};

use super::{ConfigError, ConfigWatcher, Persistable};

/// Keeps context configuration.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ContextInfo {
    pub name: String,
    pub namespace: String,
    pub kind: String,
    pub filter_history: Vec<String>,
    pub search_history: Vec<String>,
}

impl ContextInfo {
    /// Creates new [`ContextInfo`] instance.
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    /// Creates new [`ContextInfo`] instance from the [`ResourcesInfo`].
    pub fn from(info: &ResourcesInfo) -> Self {
        Self {
            name: info.context.clone(),
            namespace: info.namespace.as_str().into(),
            kind: info.kind.as_str().to_owned(),
            filter_history: Vec::new(),
            search_history: Vec::new(),
        }
    }

    /// Optionally updates `kind` and / or `namespace`.
    pub fn update(&mut self, kind: Option<String>, namespace: Option<String>) {
        if let Some(namespace) = namespace {
            self.namespace = namespace;
        }

        if let Some(kind) = kind {
            self.kind = kind;
        }
    }
}

/// Keeps context configuration for individual kube config.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct KubeConfig {
    pub current_context: Option<String>,
    pub contexts: Vec<ContextInfo>,
}

impl KubeConfig {
    /// Creates new [`KubeConfig`] instance.
    pub fn new(context: String, kind: Option<String>, namespace: Option<String>) -> Self {
        let mut new_context = ContextInfo::new(context.clone());
        new_context.update(kind, namespace);

        Self {
            current_context: Some(context),
            contexts: vec![new_context],
        }
    }
}

static EMPTY_LIST: Vec<String> = Vec::new();

/// Application history.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct History {
    pub kube_configs: HashMap<String, KubeConfig>,
    #[serde(skip_serializing)]
    current_kube_config: Option<String>,
    #[serde(skip_serializing)]
    current_hash: Option<String>,
}

impl History {
    /// Returns watcher for history.
    pub fn watcher() -> ConfigWatcher<History> {
        ConfigWatcher::new(History::default_path())
    }

    /// Loads history from a file or creates default one if the file does not exist.
    pub async fn load_or_create() -> Result<Self, ConfigError> {
        let history = Self::load(&History::default_path()).await;
        match history {
            Ok(history) => Ok(history),
            Err(ConfigError::SerializationError(_)) => Ok(Self::default()),
            Err(_) => {
                let history = Self::default();
                history.save(&History::default_path()).await?;
                Ok(history)
            },
        }
    }

    /// Returns the default history file path: `HOME/b4n/history.yaml`.
    pub fn default_path() -> PathBuf {
        match std::env::home_dir() {
            Some(path) => path.join(format!(".{}", super::APP_NAME)).join("history.yaml"),
            None => PathBuf::from("history.yaml"),
        }
    }

    /// Returns a kind stored in the history under a specific context name.
    pub fn get_kind(&self, context: &str) -> Option<&str> {
        if let Some(index) = self.context_index(context) {
            Some(&self.kube_configs[self.config_key()].contexts[index].kind)
        } else {
            None
        }
    }

    /// Returns a namespace stored in the history under a specific context name.
    pub fn get_namespace(&self, context: &str) -> Option<&str> {
        if let Some(index) = self.context_index(context) {
            Some(&self.kube_configs[self.config_key()].contexts[index].namespace)
        } else {
            None
        }
    }

    /// Gets the currently used kube config path.
    pub fn kube_config_path(&self) -> Option<&str> {
        self.current_kube_config.as_deref()
    }

    /// Sets the currently used kube config path.
    pub fn set_kube_config_path(&mut self, path: Option<String>) {
        if let Some(path) = path {
            self.current_hash = Some(calculate_hash(&path, 8));
            self.current_kube_config = Some(path);
        } else {
            self.current_hash = None;
            self.current_kube_config = None;
        }
    }

    /// Returns currently selected context name.
    pub fn current_context(&self) -> Option<&str> {
        self.current_config().and_then(|c| c.current_context.as_deref())
    }

    /// Creates or updates (if exists) context data.
    pub fn create_or_update_context(&mut self, context: String, kind: Option<String>, namespace: Option<String>) {
        if let Some(config) = self.current_config_mut() {
            if let Some(index) = config.contexts.iter().position(|c| c.name == context) {
                config.contexts[index].update(kind, namespace);
            } else {
                let mut context = ContextInfo::new(context.clone());
                context.update(kind, namespace);
                config.contexts.push(context);
            }

            config.current_context = Some(context);
        } else {
            self.kube_configs
                .insert(self.config_key().to_owned(), KubeConfig::new(context, kind, namespace));
        }
    }

    /// Gets `filter_history` from the specified `context` of the current kube config.
    pub fn get_filter_history(&mut self, context: &str) -> &[String] {
        if let Some(config) = self.current_config_mut()
            && let Some(index) = config.contexts.iter().position(|c| c.name == context)
        {
            &config.contexts[index].filter_history
        } else {
            &EMPTY_LIST
        }
    }

    /// Updates `filter_history` in the specified `context` of the current kube config.
    pub fn update_filter_history(&mut self, context: &str, filter_history: Vec<String>) {
        if let Some(config) = self.current_config_mut()
            && let Some(index) = config.contexts.iter().position(|c| c.name == context)
        {
            config.contexts[index].filter_history = filter_history;
        }
    }

    /// Gets `search_history` from the specified `context` of the current kube config.
    pub fn get_search_history(&mut self, context: &str) -> &[String] {
        if let Some(config) = self.current_config_mut()
            && let Some(index) = config.contexts.iter().position(|c| c.name == context)
        {
            &config.contexts[index].search_history
        } else {
            &EMPTY_LIST
        }
    }

    /// Updates `search_history` in the specified `context` of the current kube config.
    pub fn update_search_history(&mut self, context: &str, search_history: Vec<String>) {
        if let Some(config) = self.current_config_mut()
            && let Some(index) = config.contexts.iter().position(|c| c.name == context)
        {
            config.contexts[index].search_history = search_history;
        }
    }

    fn config_key(&self) -> &str {
        match &self.current_hash {
            Some(hash) => hash,
            None => "default",
        }
    }

    fn context_index(&self, context: &str) -> Option<usize> {
        self.kube_configs
            .get(self.config_key())
            .and_then(|c| c.contexts.iter().position(|c| c.name == context))
    }

    fn current_config(&self) -> Option<&KubeConfig> {
        self.kube_configs.get(self.config_key())
    }

    fn current_config_mut(&mut self) -> Option<&mut KubeConfig> {
        let current_key = match &self.current_hash {
            Some(hash) => hash,
            None => "default",
        };

        self.kube_configs.get_mut(current_key)
    }
}

impl Persistable<History> for History {
    async fn load(path: &Path) -> Result<History, ConfigError> {
        let mut file = File::open(path).await?;

        let mut history_str = String::new();
        file.read_to_string(&mut history_str).await?;

        Ok(serde_yaml::from_str::<History>(&history_str)?)
    }

    async fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let history_str = serde_yaml::to_string(self)?;

        let mut file = File::create(path).await?;
        file.write_all(history_str.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}
