use crate::kubernetes::client::KubernetesClient;

use super::ExecutorResult;

pub struct NewKubernetesClientCommand {
    pub context: String,
}

impl NewKubernetesClientCommand {
    /// Creates new [`NewKubernetesClientCommand`] instance.
    pub fn new(context: String) -> Self {
        Self { context }
    }

    /// Creates new kubernetes client and returns it.
    pub async fn execute(&self) -> Option<ExecutorResult> {
        if let Ok(client) = KubernetesClient::new(Some(&self.context), false).await {
            Some(ExecutorResult::KubernetesClient(client, self.context.clone()))
        } else {
            None
        }
    }
}
