use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::ui::theme::Theme;

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

pub trait Persistable<T> {
    /// Loads configuration from the default file.
    fn load() -> impl Future<Output = Result<T, ConfigError>> + Send;

    /// Saves configuration to the default file.
    fn save(&self) -> impl Future<Output = Result<(), ConfigError>> + Send;
}

/// Application configuration.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub theme: Theme,
}

impl Config {
    /// Returns watcher for configuration.
    pub fn watcher() -> ConfigWatcher<Config> {
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
}

impl Persistable<Config> for Config {
    async fn load() -> Result<Config, ConfigError> {
        let mut file = File::open(get_default_config_dir()).await?;

        let mut config_str = String::new();
        file.read_to_string(&mut config_str).await?;

        Ok(serde_yaml::from_str::<Config>(&config_str)?)
    }

    async fn save(&self) -> Result<(), ConfigError> {
        let config_str = serde_yaml::to_string(self)?;

        let mut file = File::create(get_default_config_dir()).await?;
        file.write_all(config_str.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}

fn get_default_config_dir() -> PathBuf {
    match home::home_dir() {
        Some(path) => path.join(format!(".{}", env!("CARGO_CRATE_NAME"))).join("config.yaml"),
        None => PathBuf::from("config.yaml"),
    }
}
