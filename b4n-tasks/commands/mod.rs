use b4n_config::{Config, History};
use b4n_kube::Port;
use kube::config::NamedContext;
use std::path::PathBuf;

pub use self::delete_resources::DeleteResourcesCommand;
pub use self::get_new_yaml::{GetNewResourceYamlCommand, GetNewResourceYamlError, GetNewResourceYamlResult};
pub use self::get_yaml::{GetResourceYamlCommand, ResourceYamlError, ResourceYamlResult};
pub use self::list_contexts::ListKubeContextsCommand;
pub use self::list_resource_ports::ListResourcePortsCommand;
pub use self::list_themes::ListThemesCommand;
pub use self::new_kubernetes_client::{KubernetesClientError, KubernetesClientResult, NewKubernetesClientCommand};
pub use self::save_configuration::SaveConfigurationCommand;
pub use self::set_new_yaml::{SetNewResourceYamlCommand, SetNewResourceYamlError};
pub use self::set_yaml::{SetResourceYamlAction, SetResourceYamlCommand, SetResourceYamlError, SetResourceYamlOptions};

mod delete_resources;
mod get_new_yaml;
mod get_yaml;
mod list_contexts;
mod list_resource_ports;
mod list_themes;
mod new_kubernetes_client;
mod save_configuration;
mod set_new_yaml;
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
    GetNewYaml(Box<GetNewResourceYamlCommand>),
    GetYaml(Box<GetResourceYamlCommand>),
    SetNewYaml(Box<SetNewResourceYamlCommand>),
    SetYaml(Box<SetResourceYamlCommand>),
}

/// List of all possible results from commands executed in the executor.
pub enum CommandResult {
    ContextsList(Vec<NamedContext>),
    ResourcePortsList(Vec<Port>),
    ThemesList(Vec<PathBuf>),
    KubernetesClient(Result<KubernetesClientResult, KubernetesClientError>),
    GetNewResourceYaml(Result<GetNewResourceYamlResult, GetNewResourceYamlError>),
    GetResourceYaml(Result<ResourceYamlResult, ResourceYamlError>),
    SetNewResourceYaml(Result<String, SetNewResourceYamlError>),
    SetResourceYaml(Result<String, SetResourceYamlError>),
}
