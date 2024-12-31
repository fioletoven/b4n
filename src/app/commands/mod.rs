pub use self::delete::*;
pub use self::executor::*;

mod delete;
mod executor;

/// List of all possible commands for [`BgExecutor`]
pub enum Command {
    DeleteResource(DeleteResourcesCommand),
}
