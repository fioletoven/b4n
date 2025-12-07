use b4n_kube::Namespace;
use base64::{Engine, engine};
use k8s_openapi::serde_json::Value;
use kube::api::DynamicObject;
use kube::api::PostParams;
use kube::core::GroupVersionKind;
use kube::{Client, Discovery};

use crate::commands::CommandResult;

/// Possible errors from applying or patching resource's YAML.
#[derive(thiserror::Error, Debug)]
pub enum SetNewResourceYamlError {
    /// Create is not supported for the specified resource.
    #[error("create is not supported for the specified resource")]
    CreateNotSupported,

    /// Missing apiVersion in the specified YAML.
    #[error("missing apiVersion in the specified YAML")]
    MissingApiVersion,

    /// Missing kind in the specified YAML.
    #[error("missing kind in the specified YAML")]
    MissingKind,

    /// Failed to deserialize YAML into resource.
    #[error("failed to deserialize YAML: {0}")]
    SerializationError(#[from] serde_yaml::Error),

    /// Specified group, version and kind not found.
    #[error("specified group, version and kind not found")]
    ResourceNotFound,

    /// Failed to patch or apply YAML to the Kubernetes resource.
    #[error("failed to create resource: {0}")]
    CreateError(#[from] kube::Error),
}

/// Command that apply/patch specified kubernetes resource.
pub struct SetNewResourceYamlCommand {
    yaml: String,
    client: Option<Client>,
    encode: bool,
}

impl SetNewResourceYamlCommand {
    /// Creates new [`SetNewResourceYamlCommand`] instance.
    pub fn new(yaml: String, encode: bool, client: Client) -> Self {
        Self {
            yaml,
            client: Some(client),
            encode,
        }
    }

    pub async fn execute(mut self) -> Option<CommandResult> {
        if let Some(client) = self.client.take() {
            Some(CommandResult::SetNewResourceYaml(self.create_resource(client).await))
        } else {
            None
        }
    }

    async fn create_resource(self, client: Client) -> Result<String, SetNewResourceYamlError> {
        let mut resource = serde_yaml::from_str::<DynamicObject>(&self.yaml)?;
        if self.encode {
            encode_secret_data(&mut resource);
        }

        let api_version = resource
            .types
            .as_ref()
            .map(|t| t.api_version.as_str())
            .ok_or(SetNewResourceYamlError::MissingApiVersion)?;

        let kind = resource
            .types
            .as_ref()
            .map(|t| t.kind.as_str())
            .ok_or(SetNewResourceYamlError::MissingKind)?;

        let (group, version) = match api_version.split_once('/') {
            Some((g, v)) => (g, v),
            None => ("", api_version),
        };

        let gvk = GroupVersionKind::gvk(group, version, kind);
        let discovery = Discovery::new(client.clone()).filter(&[group]).run().await?;
        if let Some((ar, cap)) = discovery.resolve_gvk(&gvk) {
            let namespace = Namespace::from(resource.metadata.namespace.as_deref());
            let api = b4n_kube::client::get_dynamic_api(&ar, &cap, client, namespace.as_option(), namespace.is_all());
            let created = api.create(&PostParams::default(), &resource).await?;

            return Ok(created.metadata.name.unwrap_or_default());
        }

        Err(SetNewResourceYamlError::ResourceNotFound)
    }
}

fn encode_secret_data(resource: &mut DynamicObject) {
    if resource.data.get("data").is_some_and(Value::is_object) {
        let engine = engine::general_purpose::STANDARD;
        for mut data in resource.data["data"].as_object_mut().unwrap().iter_mut() {
            if let Value::String(data) = &mut data.1 {
                let encoded = engine.encode(data.as_bytes());
                *data = encoded;
            }
        }
    }
}
