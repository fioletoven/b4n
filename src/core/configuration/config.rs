use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::ui::theme::Theme;

use super::ConfigWatcher;

pub const APP_NAME: &str = env!("CARGO_CRATE_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_THEME_NAME: &str = "default";

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

pub trait Persistable<T> {
    /// Returns the default configuration path.
    fn default_path() -> PathBuf;

    /// Loads configuration from the default file.
    fn load(path: &Path) -> impl Future<Output = Result<T, ConfigError>> + Send;

    /// Saves configuration to the default file.
    fn save(&self, path: &Path) -> impl Future<Output = Result<(), ConfigError>> + Send;
}

/// Kubernetes logs configuration.
#[derive(Serialize, Deserialize, Clone)]
pub struct Logs {
    pub lines: Option<i64>,
}

/// Application configuration.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub logs: Logs,
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logs: Logs { lines: Some(400) },
            theme: DEFAULT_THEME_NAME.to_owned(),
        }
    }
}

impl Config {
    /// Returns watcher for configuration.
    pub fn watcher() -> ConfigWatcher<Config> {
        ConfigWatcher::new(Config::default_path())
    }

    /// Returns path to the themes directory.
    pub fn themes_dir() -> PathBuf {
        match std::env::home_dir() {
            Some(path) => path.join(format!(".{APP_NAME}")).join("themes"),
            None => PathBuf::from("themes"),
        }
    }

    /// Loads the configuration from a file or creates a default one if the file does not exist.
    pub async fn load_or_create() -> Result<Self, ConfigError> {
        load_or_create_default(&Self::default_path()).await
    }

    /// Loads the theme specified in the configuration.\
    /// **Note**, if the theme does not exist, a default one is created.
    pub async fn load_or_create_theme(&self) -> Result<Theme, ConfigError> {
        tokio::fs::create_dir_all(Config::themes_dir()).await?;
        load_or_create_default(&self.theme_path()).await
    }

    /// Returns path to the [`Theme`] set in the configuration.
    pub fn theme_path(&self) -> PathBuf {
        let path = Config::themes_dir().join(format!("{}.yaml", self.theme));
        if path.exists() {
            path
        } else {
            Config::themes_dir().join(format!("{DEFAULT_THEME_NAME}.yaml"))
        }
    }
}

impl Persistable<Config> for Config {
    /// Returns the default configuration path: `HOME/b4n/config.yaml`.
    fn default_path() -> PathBuf {
        match std::env::home_dir() {
            Some(path) => path.join(format!(".{APP_NAME}")).join("config.yaml"),
            None => PathBuf::from("config.yaml"),
        }
    }

    async fn load(path: &Path) -> Result<Config, ConfigError> {
        let mut file = File::open(path).await?;

        let mut config_str = String::new();
        file.read_to_string(&mut config_str).await?;

        Ok(serde_yaml::from_str::<Config>(&config_str)?)
    }

    async fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let config_str = serde_yaml::to_string(self)?;

        let mut file = File::create(path).await?;
        file.write_all(config_str.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}

async fn load_or_create_default<T: Persistable<T> + Default>(path: &Path) -> Result<T, ConfigError> {
    let configuration = T::load(path).await;
    match configuration {
        Ok(configuration) => Ok(configuration),
        Err(ConfigError::SerializationError(_)) => Ok(T::default()),
        Err(_) => {
            let configuration = T::default();
            configuration.save(path).await?;
            Ok(configuration)
        },
    }
}
