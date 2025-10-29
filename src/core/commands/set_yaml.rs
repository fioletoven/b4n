use b4n_kube::{Namespace, SECRETS};
use base64::{Engine, engine};
use k8s_openapi::serde_json::Value;
use kube::{
    Api, Client,
    api::{ApiResource, DynamicObject, Patch, PatchParams},
    discovery::{ApiCapabilities, verbs},
};
use std::fmt::Display;

use crate::core::{APP_NAME, commands::CommandResult};

/// Possible errors from applying or patching resource's YAML.
#[derive(thiserror::Error, Debug)]
pub enum SetResourceYamlError {
    /// Patch is not supported for the specified resource.
    #[error("patch is not supported for the specified resource")]
    PatchNotSupported,

    /// Failed to parse YAML into Kubernetes resource.
    #[error("failed to deserialize YAML for resource '{resource}': {source}")]
    SerializationError {
        resource: String,
        #[source]
        source: serde_yaml::Error,
    },

    /// Failed to patch or apply YAML to the Kubernetes resource.
    #[error("failed to {action} resource '{resource}': {source}")]
    PatchError {
        action: SetResourceYamlAction,
        resource: String,
        #[source]
        source: kube::Error,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum SetResourceYamlAction {
    Apply,
    ForceApply,
    Patch,
}

impl Display for SetResourceYamlAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetResourceYamlAction::Apply => write!(f, "apply"),
            SetResourceYamlAction::ForceApply => write!(f, "force apply"),
            SetResourceYamlAction::Patch => write!(f, "patch"),
        }
    }
}

/// Command that apply/patch specified kubernetes resource.
pub struct SetResourceYamlCommand {
    name: String,
    namespace: Namespace,
    yaml: String,
    action: SetResourceYamlAction,
    discovery: Option<(ApiResource, ApiCapabilities)>,
    client: Option<Client>,
}

impl SetResourceYamlCommand {
    /// Creates new [`SetResourceYamlCommand`] instance.
    pub fn new(
        name: String,
        namespace: Namespace,
        yaml: String,
        action: SetResourceYamlAction,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
    ) -> Self {
        Self {
            name,
            namespace,
            yaml,
            action,
            discovery,
            client: Some(client),
        }
    }

    pub async fn execute(mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::PATCH) {
            return Some(CommandResult::SetResourceYaml(Err(SetResourceYamlError::PatchNotSupported)));
        }

        let client = b4n_kube::client::get_dynamic_api(
            &discovery.0,
            &discovery.1,
            self.client.take().expect("kubernetes client should be present"),
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        let is_secret = discovery.0.plural == SECRETS;

        Some(CommandResult::SetResourceYaml(self.save_yaml(client, is_secret).await))
    }

    async fn save_yaml(self, api: Api<DynamicObject>, encode: bool) -> Result<String, SetResourceYamlError> {
        let mut resource =
            serde_yaml::from_str::<DynamicObject>(&self.yaml).map_err(|e| SetResourceYamlError::SerializationError {
                resource: self.name.clone(),
                source: e,
            })?;

        if encode {
            encode_secret_data(&mut resource);
        }

        let (patch, patch_params) = match self.action {
            SetResourceYamlAction::Apply => (Patch::Apply(&resource), PatchParams::apply(APP_NAME)),
            SetResourceYamlAction::ForceApply => (Patch::Apply(&resource), PatchParams::apply(APP_NAME).force()),
            SetResourceYamlAction::Patch => (Patch::Merge(&resource), PatchParams::default()),
        };

        api.patch(&self.name, &patch_params, &patch)
            .await
            .map_err(|e| SetResourceYamlError::PatchError {
                action: self.action,
                resource: self.name.clone(),
                source: e,
            })?;

        Ok(self.name)
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
