use crate::kubernetes::client::list_contexts;

/// Command that reads kube config file and lists all contexts from it.
pub struct ListKubeContextsCommand {}

impl ListKubeContextsCommand {
    /// Gets all contexts from the kube config file.
    pub async fn execute(&self) -> bool {
        if let Ok(contexts) = list_contexts().await {
            true
        } else {
            false
        }
    }
}
