pub use self::config::{APP_NAME, APP_VERSION, Config, ConfigError, DEFAULT_THEME_NAME};
pub use self::history::History;
pub use self::syntax::SyntaxData;
pub use self::watcher::{ConfigWatcher, Persistable};

pub mod keys;
pub mod themes;

mod config;
mod history;
mod syntax;
mod utils;
mod watcher;
