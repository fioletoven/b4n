use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{app::ResourcesInfo, ui::theme::Theme, utils::calculate_hash};

use super::ConfigWatcher;

/// Possible errors from [`Config`] manipulation.
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    /// Cannot read/write configuration file.
    #[error("cannot read/write configuration file")]
    IoError(#[from] std::io::Error),

    /// Cannot serialize/deserialize configuration.
    #[error("cannot serialize/deserialize configuration")]
    SerializationError(#[from] serde_yaml::Error),
}

/// Keeps context configuration.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ContextInfo {
    pub name: String,
    pub namespace: String,
    pub kind: String,
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
            kind: info.kind.clone(),
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

/// Application configuration.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub kube_configs: HashMap<String, KubeConfig>,
    pub theme: Theme,
    #[serde(skip_serializing)]
    current_kube_config: Option<String>,
    #[serde(skip_serializing)]
    current_hash: Option<String>,
}

impl Config {
    /// Returns watcher for configuration.
    pub fn watcher() -> ConfigWatcher {
        ConfigWatcher::new(get_default_config_dir())
    }

    /// Loads configuration from a file or creates default one if the file does not exist.
    /// Default location for the configuration file is: `HOME/b4n/config.yaml`.
    pub async fn load_or_create() -> Result<Self, ConfigError> {
        let config = Self::load().await;
        match config {
            Ok(config) => Ok(config),
            Err(ConfigError::SerializationError(_)) => Ok(Self::default()),
            Err(_) => {
                let config = Self::default();
                config.save().await?;
                Ok(config)
            }
        }
    }

    /// Loads configuration from the default file located at `HOME/b4n/config.yaml`.
    pub async fn load() -> Result<Self, ConfigError> {
        let mut file = File::open(get_default_config_dir()).await?;

        let mut config_str = String::new();
        file.read_to_string(&mut config_str).await?;

        Ok(serde_yaml::from_str::<Config>(&config_str)?)
    }

    /// Saves configuration to the default file located at `HOME/b4n/config.yaml`.
    pub async fn save(&self) -> Result<(), ConfigError> {
        let config_str = serde_yaml::to_string(self)?;

        let mut file = File::create(get_default_config_dir()).await?;
        file.write_all(config_str.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }

    /// Returns a kind stored in the configuration under a specific context name.
    pub fn get_kind(&self, context: &str) -> Option<&str> {
        if let Some(index) = self.context_index(context) {
            Some(&self.kube_configs[self.config_key()].contexts[index].kind)
        } else {
            None
        }
    }

    /// Returns a namespace stored in the configuration under a specific context name.
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

fn get_default_config_dir() -> PathBuf {
    match home::home_dir() {
        Some(path) => path.join(format!(".{}", env!("CARGO_CRATE_NAME"))).join("config.yaml"),
        None => PathBuf::from("config.yaml"),
    }
}
