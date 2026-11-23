use b4n_common::{DEFAULT_ERROR_DURATION, DEFAULT_MESSAGE_DURATION, NotificationSink};
use k8s_openapi::api::core::v1::Node;
use k8s_openapi::serde_json::json;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client};

use crate::commands::CommandResult;

/// Command that changes whether the scheduler can place new pods on a node.
pub struct CordonNodeCommand {
    client: Client,
    name: String,
    cordon: bool,
    sink: NotificationSink,
}

impl CordonNodeCommand {
    /// Creates new [`CordonNodeCommand`] instance.
    pub fn new(client: Client, name: String, cordon: bool, sink: NotificationSink) -> Self {
        Self {
            client,
            name,
            cordon,
            sink,
        }
    }

    /// Changes whether the scheduler can place new pods on a node.
    pub async fn execute(self) -> Option<CommandResult> {
        let action = if self.cordon { "cordon" } else { "uncordon" };

        if self.name.is_empty() {
            let msg = format!("Cannot {action} node: node name is not provided");
            tracing::error!("{}", msg);
            self.sink.show_error(msg, DEFAULT_ERROR_DURATION);

            return None;
        }

        match cordon_node(self.client, &self.name, self.cordon).await {
            Ok(()) => {
                let message = format!("Node '{}' {action}ed", self.name);
                tracing::info!("{}", message);
                self.sink.show_info(message, DEFAULT_MESSAGE_DURATION);
            },
            Err(error) => {
                let msg = format!("Cannot {action} node '{}': {error}", self.name);
                tracing::error!("{}", msg);
                self.sink.show_error(msg, DEFAULT_ERROR_DURATION);
            },
        }

        None
    }
}

/// Sets unschedulable property in node's spec to the specified value.
pub async fn cordon_node(client: Client, name: &str, cordon: bool) -> Result<(), kube::Error> {
    let api: Api<Node> = Api::all(client);
    let patch = json!({ "spec": { "unschedulable": cordon } });

    api.patch(name, &PatchParams::default(), &Patch::Merge(&patch)).await?;

    Ok(())
}
