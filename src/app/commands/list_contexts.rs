use tracing::error;

use crate::kubernetes::client::list_contexts;

use super::CommandResult;

/// Command that reads kube config file and lists all contexts from it.
pub struct ListKubeContextsCommand;

impl ListKubeContextsCommand {
    /// Gets all contexts from the kube config file.
    pub async fn execute(&self) -> Option<CommandResult> {
        match list_contexts().await {
            Ok(contexts) => Some(CommandResult::ContextsList(contexts)),
            Err(error) => {
                error!("Cannot read contexts list: {}", error);
                None
            }
        }
    }
}
