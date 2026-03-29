use anyhow::Result;
use b4n_common::calculate_hash;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Handle;

use crate::{ConfigError, ConfigWatcher, Persistable};

/// Keeps context configuration.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ContextInfo {
    pub name: String,
    pub namespace: String,
    pub kind: String,
    pub filter_history: Vec<HistoryItem>,
    pub search_history: Vec<HistoryItem>,
    pub namespace_history: Vec<HistoryItem>,
}

impl ContextInfo {
    /// Creates new [`ContextInfo`] instance.
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
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

static EMPTY_LIST: Vec<HistoryItem> = Vec::new();

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
    pub fn watcher(runtime: Handle) -> ConfigWatcher<History> {
        ConfigWatcher::new(runtime, History::default_path())
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
    pub fn filter_history(&self, context: &str) -> &[HistoryItem] {
        if let Some(config) = self.current_config()
            && let Some(index) = config.contexts.iter().position(|c| c.name == context)
        {
            &config.contexts[index].filter_history
        } else {
            &EMPTY_LIST
        }
    }

    /// Adds new item to `filter_history` in the specified `context` of the current kube config.
    pub fn add_filter_history_item(&mut self, context: &str, item: HistoryItem, max_list_size: usize) -> bool {
        if let Some(config) = self.current_config_mut()
            && let Some(context) = config.contexts.iter_mut().find(|c| c.name == context)
        {
            add_history_item(&mut context.filter_history, item, max_list_size)
        } else {
            false
        }
    }

    /// Removes an item from `filter_history` in the specified `context` of the current kube config.
    pub fn remove_filter_history_item(&mut self, context: &str, item: &str) -> Option<HistoryItem> {
        if let Some(config) = self.current_config_mut()
            && let Some(context) = config.contexts.iter_mut().find(|c| c.name == context)
        {
            del_history_item(&mut context.filter_history, item)
        } else {
            None
        }
    }

    /// Gets `search_history` from the specified `context` of the current kube config.
    pub fn search_history(&self, context: &str) -> &[HistoryItem] {
        if let Some(config) = self.current_config()
            && let Some(index) = config.contexts.iter().position(|c| c.name == context)
        {
            &config.contexts[index].search_history
        } else {
            &EMPTY_LIST
        }
    }

    /// Adds new item to `search_history` in the specified `context` of the current kube config.
    pub fn add_search_history_item(&mut self, context: &str, item: HistoryItem, max_list_size: usize) -> bool {
        if let Some(config) = self.current_config_mut()
            && let Some(context) = config.contexts.iter_mut().find(|c| c.name == context)
        {
            add_history_item(&mut context.search_history, item, max_list_size)
        } else {
            false
        }
    }

    /// Removes an item from `search_history` in the specified `context` of the current kube config.
    pub fn remove_search_history_item(&mut self, context: &str, item: &str) -> Option<HistoryItem> {
        if let Some(config) = self.current_config_mut()
            && let Some(context) = config.contexts.iter_mut().find(|c| c.name == context)
        {
            del_history_item(&mut context.search_history, item)
        } else {
            None
        }
    }

    /// Gets `namespace_history` from the specified `context` of the current kube config.
    pub fn namespace_history(&self, context: &str) -> &[HistoryItem] {
        if let Some(config) = self.current_config()
            && let Some(index) = config.contexts.iter().position(|c| c.name == context)
        {
            &config.contexts[index].namespace_history
        } else {
            &EMPTY_LIST
        }
    }

    /// Adds new item to `namespace_history` in the specified `context` of the current kube config.
    pub fn add_namespace_history_item(&mut self, context: &str, item: HistoryItem, max_list_size: usize) -> bool {
        if let Some(config) = self.current_config_mut()
            && let Some(context) = config.contexts.iter_mut().find(|c| c.name == context)
        {
            add_history_item(&mut context.namespace_history, item, max_list_size)
        } else {
            false
        }
    }

    /// Removes an item from `namespace_history` in the specified `context` of the current kube config.
    pub fn remove_namespace_history_item(&mut self, context: &str, item: &str) -> Option<HistoryItem> {
        if let Some(config) = self.current_config_mut()
            && let Some(context) = config.contexts.iter_mut().find(|c| c.name == context)
        {
            del_history_item(&mut context.namespace_history, item)
        } else {
            None
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
    /// Returns the default history file path: `HOME/b4n/history.yaml`.
    fn default_path() -> PathBuf {
        match std::env::home_dir() {
            Some(path) => path.join(format!(".{}", super::APP_NAME)).join("history.yaml"),
            None => PathBuf::from("history.yaml"),
        }
    }

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

/// History item with creation time.
#[derive(Clone)]
pub struct HistoryItem {
    pub value: String,
    pub creation_time: SystemTime,
}

impl std::fmt::Display for HistoryItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}::{}",
            self.value,
            self.creation_time
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        )
    }
}

impl Serialize for HistoryItem {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HistoryItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct HistoryItemVisitor;

        impl<'de> Visitor<'de> for HistoryItemVisitor {
            type Value = HistoryItem;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string in the format `value::timestamp`")
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                let (value, creation_time) = match s.rsplit_once("::") {
                    Some((value, timestamp_str)) => {
                        let timestamp: u64 = timestamp_str.parse().map_err(|_| de::Error::custom("invalid timestamp"))?;
                        (value.to_string(), UNIX_EPOCH + std::time::Duration::from_secs(timestamp))
                    },
                    None => (s.to_string(), SystemTime::now()),
                };

                Ok(HistoryItem { value, creation_time })
            }
        }

        deserializer.deserialize_str(HistoryItemVisitor)
    }
}

impl From<&str> for HistoryItem {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_owned(),
            creation_time: SystemTime::now(),
        }
    }
}

fn add_history_item(list: &mut Vec<HistoryItem>, item: HistoryItem, max_list_size: usize) -> bool {
    if !item.value.is_empty() && list.iter().all(|i| i.value != item.value) {
        list.push(item);

        if list.len() > max_list_size {
            let index = list
                .iter()
                .enumerate()
                .min_by_key(|(_, i)| i.creation_time)
                .map(|(index, _)| index);
            if let Some(index) = index {
                list.remove(index);
            }
        }

        true
    } else {
        false
    }
}

fn del_history_item(list: &mut Vec<HistoryItem>, item: &str) -> Option<HistoryItem> {
    if !item.is_empty()
        && let Some(idx) = list.iter().position(|i| i.value == item)
    {
        Some(list.remove(idx))
    } else {
        None
    }
}
