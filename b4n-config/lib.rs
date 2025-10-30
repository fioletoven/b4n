pub use self::config::{APP_NAME, APP_VERSION, Config, DEFAULT_THEME_NAME};
pub use self::errors::ConfigError;
pub use self::history::History;
pub use self::syntax::SyntaxData;
pub use self::watcher::{ConfigWatcher, Persistable};

pub mod keys;
pub mod themes;

mod config;
mod errors;
mod history;
mod syntax;
mod watcher;
