use kube::config::NamedContext;

pub use self::delete_resources::*;
pub use self::list_contexts::*;
pub use self::new_kubernetes_client::*;
pub use self::save_configuration::*;

mod delete_resources;
mod list_contexts;
mod new_kubernetes_client;
mod save_configuration;

/// List of all possible commands for [BgExecutor](super::BgExecutor).
pub enum Command {
    ListKubeContexts(ListKubeContextsCommand),
    NewKubernetesClient(Box<NewKubernetesClientCommand>),
    SaveConfiguration(Box<SaveConfigurationCommand>),
    DeleteResource(Box<DeleteResourcesCommand>),
}

/// List of all possible results from commands executed in the executor.
pub enum CommandResult {
    ContextsList(Vec<NamedContext>),
    KubernetesClient(Result<KubernetesClientResult, KubernetesClientError>),
}
