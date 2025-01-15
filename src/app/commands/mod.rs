pub use self::cmd_delete_resources::*;
pub use self::cmd_list_contexts::*;
pub use self::cmd_save_configuration::*;
pub use self::executor::*;

mod cmd_delete_resources;
mod cmd_list_contexts;
mod cmd_save_configuration;
mod executor;

/// List of all possible commands for [`BgExecutor`]
pub enum Command {
    ListKubeContexts(ListKubeContextsCommand),
    SaveConfiguration(SaveConfigurationCommand),
    DeleteResource(DeleteResourcesCommand),
}
