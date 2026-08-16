use anyhow::Result;
use crossterm::style::Stylize;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Handle;

use crate::History;
use crate::themes::{TextColors, Theme};
use crate::{ConfigWatcher, Persistable, keys::KeyBindings, utils::sorted_map};

pub const APP_NAME: &str = "b4n";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_THEME_NAME: &str = "default";

static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Possible errors from configuration files manipulation.
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    /// Cannot find configuration file.
    #[error("configuration file not found")]
    NotFound,

    /// Cannot read/write configuration file.
    #[error("cannot read/write configuration file")]
    IoError(#[from] std::io::Error),

    /// Cannot serialize configuration.
    #[error("cannot serialize configuration")]
    SerializationError(#[from] serde_saphyr::ser::Error),

    /// Cannot deserialize configuration.
    #[error("cannot deserialize configuration")]
    DeserializationError(#[from] serde_saphyr::Error),
}

/// Kubernetes logs configuration.
#[derive(Serialize, Deserialize, Clone)]
pub struct Logs {
    pub lines: Option<i64>,
    pub timestamps: Option<bool>,
}

impl Default for Logs {
    fn default() -> Self {
        Self {
            lines: Some(800),
            timestamps: Some(true),
        }
    }
}

/// Terminal configuration used in shell, attach and plugin views.
#[derive(Serialize, Deserialize, Clone)]
pub struct Terminal {
    pub system_cursor: Option<bool>,
    pub scrollback_lines: Option<usize>,
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            system_cursor: Some(false),
            scrollback_lines: Some(1_000),
        }
    }
}

/// Application configuration.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub logs: Logs,

    #[serde(default = "default_mouse")]
    pub mouse: bool,

    #[serde(default)]
    pub terminal: Terminal,

    #[serde(default = "default_theme_name")]
    pub theme: String,

    #[serde(default = "default_images_list")]
    pub debug_images: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<HashMap<String, TextColors>>,

    #[serde(default = "default_aliases")]
    #[serde(serialize_with = "sorted_map")]
    pub aliases: HashMap<String, String>,

    pub key_bindings: Option<KeyBindings>,
}

fn default_mouse() -> bool {
    true
}

fn default_theme_name() -> String {
    DEFAULT_THEME_NAME.to_owned()
}

fn default_images_list() -> Vec<String> {
    vec!["busybox".to_string(), "alpine".to_string(), "nicolaka/netshoot".to_string()]
}

fn default_aliases() -> HashMap<String, String> {
    [
        ("clusterrolebindings", "crb"),
        ("clusterroles", "cr"),
        ("configmaps", "cm"),
        ("customresourcedefinitions", "crd"),
        ("daemonsets", "ds,dms"),
        ("namespace", "nn"),
        ("namespaces", "ns,na,nam"),
        ("networkpolicies", "np"),
        ("persistentvolumeclaims", "pvc"),
        ("persistentvolumes", "pv"),
        ("pods", "pp"),
        ("services", "svc"),
        ("statefulsets", "ss,sts"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logs: Logs::default(),
            mouse: default_mouse(),
            terminal: Terminal::default(),
            theme: default_theme_name(),
            debug_images: default_images_list(),
            contexts: None,
            key_bindings: Some(KeyBindings::default()),
            aliases: default_aliases(),
        }
    }
}

impl Config {
    /// Initialises project directories.\
    /// **Note** that it must be called once on app start.
    pub fn init_dirs(create: bool) -> Result<()> {
        let (config_dir, data_dir) = if let Some(dirs) = ProjectDirs::from("", "", APP_NAME) {
            (dirs.config_local_dir().to_path_buf(), dirs.data_local_dir().to_path_buf())
        } else if let Some(home) = std::env::home_dir() {
            let app_dir = home.join(format!(".{APP_NAME}"));
            (app_dir.clone(), app_dir)
        } else {
            let app_dir = PathBuf::from(".");
            (app_dir.clone(), app_dir)
        };

        if create {
            std::fs::create_dir_all(&config_dir)?;
            std::fs::create_dir_all(&data_dir)?;
        }

        let _ = CONFIG_PATH.set(config_dir.join("config.yaml"));
        let _ = DATA_DIR.set(data_dir);

        Ok(())
    }

    /// Prints configuration paths used by the application.
    pub fn print_dirs(kube_config: Option<PathBuf>) {
        println!("{}:     {}", "config".cyan(), Self::config_path().display());
        println!("{}:    {}", "history".cyan(), History::default_path().display());
        println!("{}:       {}", "logs".cyan(), Self::data_dir().join("logs").display());
        println!("{}:     {}", "themes".cyan(), Self::themes_dir().display());
        println!("{}:    {}", "plugins".cyan(), Self::plugins_dir().display());
        if let Some(kube_config) = kube_config {
            println!("{}: {}", "kubeconfig".cyan(), kube_config.display());
        } else {
            println!("{}: {}", "kubeconfig".cyan(), "not found".grey());
        }
    }

    /// Returns path to the configuration file.
    pub fn config_path() -> &'static Path {
        CONFIG_PATH.get().expect("init_dirs was not called")
    }

    /// Retruns path to the data directory.
    pub fn data_dir() -> &'static Path {
        DATA_DIR.get().expect("init_dirs was not called")
    }

    /// Returns path to the themes directory.
    pub fn themes_dir() -> PathBuf {
        Self::data_dir().join("themes")
    }

    /// Returns path to the plugins directory.
    pub fn plugins_dir() -> PathBuf {
        Self::data_dir().join("plugins")
    }

    /// Returns watcher for configuration.
    pub fn watcher(runtime: Handle) -> ConfigWatcher<Config> {
        ConfigWatcher::new(runtime, Config::default_path())
    }

    /// Loads the configuration from a file or creates a default one if the file does not exist.
    pub async fn load_or_create() -> (Self, Option<ConfigError>) {
        load_configuration(Self::config_path(), false, false).await
    }

    /// Loads the theme specified in the configuration.
    pub async fn load_theme(&self) -> (Theme, Option<ConfigError>) {
        let theme_path = self.theme_path();
        load_configuration(&theme_path, true, self.is_default_theme()).await
    }

    /// Returns path to the [`Theme`] set in the configuration.
    pub fn theme_path(&self) -> PathBuf {
        Config::themes_dir().join(format!("{}.yaml", self.theme))
    }

    /// Returns `true` if the currently set theme is a default one.
    pub fn is_default_theme(&self) -> bool {
        self.theme == default_theme_name()
    }
}

impl Persistable<Config> for Config {
    /// Returns the default configuration path.
    fn default_path() -> PathBuf {
        Self::config_path().to_path_buf()
    }

    async fn load(path: &Path) -> Result<Config, ConfigError> {
        let mut file = File::open(path).await?;

        let mut config_str = String::new();
        file.read_to_string(&mut config_str).await?;

        Ok(serde_saphyr::from_str::<Config>(&config_str)?)
    }

    async fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let config_str = serde_saphyr::to_string(self)?;

        let mut file = File::create(path).await?;
        file.write_all(config_str.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}

async fn load_configuration<T: Persistable<T> + Default>(
    path: &Path,
    is_theme: bool,
    is_default: bool,
) -> (T, Option<ConfigError>) {
    let kind = if is_theme { "theme" } else { "config" };
    let configuration = T::load(path).await;
    match configuration {
        Ok(configuration) => (configuration, None),
        Err(ConfigError::DeserializationError(error)) => {
            tracing::error!("Cannot deserialize {}: {}", kind, error);
            (T::default(), Some(ConfigError::DeserializationError(error)))
        },
        Err(error) => {
            let configuration = T::default();
            if !is_default {
                tracing::error!("Cannot load {}: {}", kind, error);
            }
            if !is_theme && let Err(error) = configuration.save(path).await {
                tracing::error!("Cannot save {}: {}", kind, error);
            }
            (configuration, Some(error))
        },
    }
}
