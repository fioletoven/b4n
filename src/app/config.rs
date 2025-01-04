use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self},
    path::PathBuf,
};

use crate::ui::theme::Theme;

/// Possible errors from [`Config`] manipulation
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    /// Cannot read configuration file
    #[error("cannot read configuration file")]
    FileReadError,

    /// Cannot write configuration file
    #[error("cannot write configuration file")]
    FileWriteError,

    /// Cannot deserialize configuration
    #[error("cannot deserialize configuration")]
    DeserializeError,

    /// Cannot serialize configuration
    #[error("cannot serialize configuration")]
    SerializeError,
}

/// Application configuration
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub theme: Theme,
}

impl Config {
    /// Loads configuration a from file or creates default one if the file does not exist.  
    /// Default location for the configuration file is: `HOME/b4n/config.yaml`
    pub fn load_or_create() -> Result<Self, ConfigError> {
        let config = Self::load();
        match config {
            Ok(config) => Ok(config),
            Err(ConfigError::DeserializeError) => Ok(Self::default()),
            Err(_) => {
                let config = Self::default();
                config.save()?;
                Ok(config)
            }
        }
    }

    /// Loads configuration from the default file located at `HOME/b4n/config.yaml`
    pub fn load() -> Result<Self, ConfigError> {
        let Ok(config_str) = fs::read_to_string(get_default_config_dir()) else {
            return Err(ConfigError::FileReadError);
        };
        let Ok(config) = serde_yaml::from_str::<Config>(&config_str) else {
            return Err(ConfigError::DeserializeError);
        };

        Ok(config)
    }

    /// Saves configuration to the default file located at `HOME/b4n/config.yaml`
    pub fn save(&self) -> Result<(), ConfigError> {
        let Ok(config_str) = serde_yaml::to_string(self) else {
            return Err(ConfigError::SerializeError);
        };

        if fs::write(get_default_config_dir(), config_str).is_err() {
            return Err(ConfigError::FileWriteError);
        }

        Ok(())
    }
}

fn get_default_config_dir() -> PathBuf {
    match home::home_dir() {
        Some(mut path) => {
            path.push(format!(".{}/config.yaml", env!("CARGO_CRATE_NAME")));
            path
        }
        None => PathBuf::from("config.yaml"),
    }
}
