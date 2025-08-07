use kube::config::NamedContext;
use std::path::PathBuf;

use crate::kubernetes::resources::Port;

pub use self::delete_resources::*;
pub use self::get_yaml::*;
pub use self::list_contexts::*;
pub use self::list_resource_ports::*;
pub use self::list_themes::*;
pub use self::new_kubernetes_client::*;
pub use self::save_history::*;

mod delete_resources;
mod get_yaml;
mod list_contexts;
mod list_resource_ports;
mod list_themes;
mod new_kubernetes_client;
mod save_history;

/// List of all possible commands for [`BgExecutor`](super::BgExecutor).
pub enum Command {
    ListKubeContexts(ListKubeContextsCommand),
    ListResourcePorts(Box<ListResourcePortsCommand>),
    ListThemes(ListThemesCommand),
    NewKubernetesClient(Box<NewKubernetesClientCommand>),
    SaveHistory(Box<SaveHistoryCommand>),
    DeleteResource(Box<DeleteResourcesCommand>),
    GetYaml(Box<GetResourceYamlCommand>),
}

/// List of all possible results from commands executed in the executor.
pub enum CommandResult {
    ContextsList(Vec<NamedContext>),
    ResourcePortsList(Vec<Port>),
    ThemesList(Vec<PathBuf>),
    KubernetesClient(Result<KubernetesClientResult, KubernetesClientError>),
    ResourceYaml(Result<ResourceYamlResult, ResourceYamlError>),
}
