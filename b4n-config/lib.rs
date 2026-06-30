pub use self::config::{APP_NAME, APP_VERSION, Config, ConfigError, DEFAULT_THEME_NAME};
pub use self::history::{History, HistoryItem};
pub use self::plugins::{Plugin, PluginError, PluginRef, Plugins, PluginsWatcher};
pub use self::syntax::SyntaxData;
pub use self::watcher::{ConfigWatcher, Persistable};

pub mod keys;
pub mod themes;

mod config;
mod history;
mod plugins;
mod syntax;
mod utils;
mod watcher;
