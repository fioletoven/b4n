use tracing::error;

use crate::kubernetes::client::list_contexts;

use super::ExecutorResult;

/// Command that reads kube config file and lists all contexts from it.
pub struct ListKubeContextsCommand {}

impl ListKubeContextsCommand {
    /// Gets all contexts from the kube config file.
    pub async fn execute(&self) -> Option<ExecutorResult> {
        match list_contexts().await {
            Ok(contexts) => Some(ExecutorResult::ContextsList(contexts)),
            Err(error) => {
                error!("Cannot read contexts list: {}", error);
                None
            }
        }
    }
}
