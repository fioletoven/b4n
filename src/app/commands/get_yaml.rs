use kube::{
    api::ApiResource,
    discovery::{verbs, ApiCapabilities},
    Client, ResourceExt,
};

use crate::kubernetes::{self, Namespace};

use super::CommandResult;

/// Command that returns YAML representation of specified kubernetes resource.
pub struct GetYamlCommand {
    pub name: String,
    pub namespace: Namespace,
    pub discovery: Option<(ApiResource, ApiCapabilities)>,
    pub client: Client,
}

impl GetYamlCommand {
    /// Creates new [`GetYamlCommand`] instance.
    pub fn new(name: String, namespace: Namespace, discovery: Option<(ApiResource, ApiCapabilities)>, client: Client) -> Self {
        Self {
            name,
            namespace,
            discovery,
            client,
        }
    }

    /// Returns YAML representation of the kubernetes resource.
    pub async fn execute(&mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::GET) {
            return None;
        }

        let client = kubernetes::client::get_dynamic_api(
            discovery.0.clone(),
            discovery.1.clone(),
            self.client.clone(),
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        if let Ok(mut resource) = client.get(&self.name).await {
            resource.managed_fields_mut().clear();
            if let Ok(mut resource) = serde_yaml::to_string(&resource) {
                if let Some(index) = resource.find("\n  managedFields: []\n") {
                    resource.replace_range(index + 1..index + 21, "");
                }

                return Some(CommandResult::ResourceYaml(resource));
            }
        }

        None
    }
}
