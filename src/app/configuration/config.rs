use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{app::ResourcesInfo, ui::theme::Theme};

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
            namespace: info.namespace.clone(),
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

/// Application configuration.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub current_context: Option<String>,
    pub contexts: Vec<ContextInfo>,
    pub theme: Theme,
}

impl Config {
    /// Returns watcher for configuration.
    pub fn watcher() -> ConfigWatcher {
        ConfigWatcher::new(get_default_config_dir())
    }

    /// Loads configuration a from file or creates default one if the file does not exist.  
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

    /// Searches for a context in a configuration, returning its index.
    pub fn context_index(&self, context: &str) -> Option<usize> {
        self.contexts.iter().position(|c| c.name == context)
    }

    /// Returns a kind stored in the configuration under a specific context name.
    pub fn get_kind(&self, context: &str) -> Option<&str> {
        if let Some(index) = self.context_index(context) {
            Some(&self.contexts[index].kind)
        } else {
            None
        }
    }

    /// Returns a namespace stored in the configuration under a specific context name.
    pub fn get_namespace(&self, context: &str) -> Option<&str> {
        if let Some(index) = self.context_index(context) {
            Some(&self.contexts[index].namespace)
        } else {
            None
        }
    }
}

fn get_default_config_dir() -> PathBuf {
    match home::home_dir() {
        Some(path) => path.join(format!(".{}", env!("CARGO_CRATE_NAME"))).join("config.yaml"),
        None => PathBuf::from("config.yaml"),
    }
}
