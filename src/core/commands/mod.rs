use kube::config::NamedContext;
use std::path::PathBuf;

use crate::core::Config;
use crate::core::History;
use crate::kubernetes::resources::Port;

pub use self::delete_resources::*;
pub use self::get_yaml::*;
pub use self::list_contexts::*;
pub use self::list_resource_ports::*;
pub use self::list_themes::*;
pub use self::new_kubernetes_client::*;
pub use self::save_configuration::*;
pub use self::set_yaml::*;

mod delete_resources;
mod get_yaml;
mod list_contexts;
mod list_resource_ports;
mod list_themes;
mod new_kubernetes_client;
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
    GetYaml(Box<GetResourceYamlCommand>),
    SetYaml(Box<SetResourceYamlCommand>),
}

/// List of all possible results from commands executed in the executor.
pub enum CommandResult {
    ContextsList(Vec<NamedContext>),
    ResourcePortsList(Vec<Port>),
    ThemesList(Vec<PathBuf>),
    KubernetesClient(Result<KubernetesClientResult, KubernetesClientError>),
    GetResourceYaml(Result<ResourceYamlResult, ResourceYamlError>),
    SetResourceYaml(Result<String, SetResourceYamlError>),
}
