use crate::kubernetes::resources::Context;

pub use self::cmd_delete_resources::*;
pub use self::cmd_list_contexts::*;
pub use self::cmd_save_configuration::*;
pub use self::executor::*;

mod cmd_delete_resources;
mod cmd_list_contexts;
mod cmd_save_configuration;
mod executor;

/// List of all possible commands for [`BgExecutor`]
pub enum ExecutorCommand {
    ListKubeContexts(ListKubeContextsCommand),
    SaveConfiguration(SaveConfigurationCommand),
    DeleteResource(DeleteResourcesCommand),
}

/// List of all possible results from commands executed in the executor.
pub enum ExecutorResult {
    ContextsList(Vec<Context>),
}
