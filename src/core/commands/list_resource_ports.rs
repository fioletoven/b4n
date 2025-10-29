use b4n_kube::ResourceRef;
use k8s_openapi::serde_json::Value;
use kube::{
    Client,
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, verbs},
};

use crate::{
    core::commands::CommandResult,
    kubernetes::resources::{Port, PortProtocol},
};

/// Command that gets a list of ports for the specified kubernetes resource.
pub struct ListResourcePortsCommand {
    resource: ResourceRef,
    discovery: Option<(ApiResource, ApiCapabilities)>,
    client: Client,
}

impl ListResourcePortsCommand {
    /// Creates new [`ListResourcePortsCommand`] instance.
    pub fn new(resource: ResourceRef, discovery: Option<(ApiResource, ApiCapabilities)>, client: Client) -> Self {
        Self {
            resource,
            discovery,
            client,
        }
    }

    /// Returns the list of the resource's ports.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if self.resource.name.is_none() || !discovery.1.supports_operation(verbs::GET) {
            return None;
        }

        let client = b4n_kube::client::get_dynamic_api(
            &discovery.0,
            &discovery.1,
            self.client,
            self.resource.namespace.as_option(),
            self.resource.namespace.is_all(),
        );

        match client.get(self.resource.name.as_deref().unwrap_or_default()).await {
            Ok(resource) => Some(CommandResult::ResourcePortsList(list_ports(&self.resource, &resource))),
            Err(_) => None,
        }
    }
}

fn list_ports(r: &ResourceRef, resource: &DynamicObject) -> Vec<Port> {
    if r.is_container() {
        let ports = resource.data["spec"]["containers"]
            .as_array()
            .and_then(|c| get_container_ports(r.container.as_deref().unwrap_or_default(), c));
        if let Some(ports) = ports {
            return ports;
        }
    }

    Vec::default()
}

fn get_container_ports(name: &str, containers: &[Value]) -> Option<Vec<Port>> {
    containers
        .iter()
        .find(|c| c["name"].as_str().unwrap_or_default() == name)
        .and_then(|c| c["ports"].as_array())
        .map(|p| {
            p.iter()
                .map(|p| Port {
                    port: p["containerPort"].as_u64().map_or(0, |p| u16::try_from(p).unwrap_or(0)),
                    name: p["name"].as_str().unwrap_or_default().to_owned(),
                    protocol: PortProtocol::from(p["protocol"].as_str()),
                })
                .collect::<Vec<_>>()
        })
}
