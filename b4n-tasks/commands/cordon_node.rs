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
            name,
            cordon,
            client,
            sink,
        }
    }

    /// Changes whether the scheduler can place new pods on a node.
    pub async fn execute(self) -> Option<CommandResult> {
        let action = if self.cordon { "cordon" } else { "uncordon" };

        if self.name.is_empty() {
            let message = format!("Cannot {action} node: node name is not provided");
            tracing::error!("{}", message);
            self.sink.show_error(message, DEFAULT_ERROR_DURATION);

            return None;
        }

        let api: Api<Node> = Api::all(self.client);
        let patch = json!({ "spec": { "unschedulable": self.cordon } });

        match api.patch(&self.name, &PatchParams::default(), &Patch::Merge(&patch)).await {
            Ok(_) => {
                let message = format!("Node '{}' {action}ed", self.name);
                tracing::info!("{}", message);
                self.sink.show_info(message, DEFAULT_MESSAGE_DURATION);
            },
            Err(error) => {
                let message = format!("Cannot {action} node '{}': {error}", self.name);
                tracing::error!("{}", message);
                self.sink.show_error(message, DEFAULT_ERROR_DURATION);
            },
        }

        None
    }
}
