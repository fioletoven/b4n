pub use self::delete::*;
pub use self::executor::*;
pub use self::save_config::*;

mod delete;
mod executor;
mod save_config;

/// List of all possible commands for [`BgExecutor`]
pub enum Command {
    SaveConfiguration(SaveConfigCommand),
    DeleteResource(DeleteResourcesCommand),
}
