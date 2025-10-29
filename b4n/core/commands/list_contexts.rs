use b4n_kube::client::list_contexts;
use tracing::error;

use super::CommandResult;

/// Command that reads kube config file and lists all contexts from it.
pub struct ListKubeContextsCommand {
    pub kube_config_path: Option<String>,
}

impl ListKubeContextsCommand {
    /// Gets all contexts from the kube config file.
    pub async fn execute(&self) -> Option<CommandResult> {
        match list_contexts(self.kube_config_path.as_deref()).await {
            Ok(contexts) => Some(CommandResult::ContextsList(contexts)),
            Err(error) => {
                error!("Cannot read contexts list: {}", error);
                None
            },
        }
    }
}
