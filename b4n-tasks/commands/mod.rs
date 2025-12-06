use b4n_config::{Config, History};
use b4n_kube::Port;
use kube::config::NamedContext;
use std::path::PathBuf;

pub use self::delete_resources::DeleteResourcesCommand;
pub use self::get_yaml::{GetResourceYamlCommand, ResourceYamlError, ResourceYamlResult};
pub use self::list_contexts::ListKubeContextsCommand;
pub use self::list_resource_ports::ListResourcePortsCommand;
pub use self::list_themes::ListThemesCommand;
pub use self::new_kubernetes_client::{KubernetesClientError, KubernetesClientResult, NewKubernetesClientCommand};
pub use self::new_yaml::{NewResourceYamlCommand, NewResourceYamlError, NewResourceYamlResult};
pub use self::save_configuration::SaveConfigurationCommand;
pub use self::set_yaml::{SetResourceYamlAction, SetResourceYamlCommand, SetResourceYamlError};

mod delete_resources;
mod get_yaml;
mod list_contexts;
mod list_resource_ports;
mod list_themes;
mod new_kubernetes_client;
mod new_yaml;
mod save_configuration;
mod set_yaml;

/// List of all possible commands for [`BgExecutor`](super::BgExecutor).
pub enum Command {
    ListKubeContexts(ListKubeContextsCommand),
    ListResourcePorts(Box<ListResourcePortsCommand>),
    ListThemes(ListThemesCommand),
    NewKubernetesClient(Box<NewKubernetesClientCommand>),
    SaveConfig(Box<SaveConfigurationCommand<Config>>),
    SaveHistory(Box<SaveConfigurationCommand<History>>),
    DeleteResource(Box<DeleteResourcesCommand>),
    NewYaml(Box<NewResourceYamlCommand>),
    GetYaml(Box<GetResourceYamlCommand>),
    SetYaml(Box<SetResourceYamlCommand>),
}

/// List of all possible results from commands executed in the executor.
pub enum CommandResult {
    ContextsList(Vec<NamedContext>),
    ResourcePortsList(Vec<Port>),
    ThemesList(Vec<PathBuf>),
    KubernetesClient(Result<KubernetesClientResult, KubernetesClientError>),
    NewResourceYaml(Result<NewResourceYamlResult, NewResourceYamlError>),
    GetResourceYaml(Result<ResourceYamlResult, ResourceYamlError>),
    SetResourceYaml(Result<String, SetResourceYamlError>),
}
