pub use self::colors::{LineColors, TextColors, from_syntect_color, to_syntect_color};
pub use self::config::{APP_NAME, APP_VERSION, Config, DEFAULT_THEME_NAME};
pub use self::errors::ConfigError;
pub use self::history::History;
pub use self::watcher::{ConfigWatcher, Persistable};

pub mod keys;
pub mod theme;

mod colors;
mod config;
mod errors;
mod history;
mod watcher;
