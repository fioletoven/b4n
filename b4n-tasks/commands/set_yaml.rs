use b4n_kube::utils::{can_patch_status, encode_secret_data};
use b4n_kube::{Namespace, SECRETS};
use kube::api::{ApiResource, DynamicObject, Patch, PatchParams};
use kube::discovery::{ApiCapabilities, verbs};
use kube::{Api, Client};
use std::fmt::Display;

use crate::commands::CommandResult;

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

/// Represents patch action.
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

/// Holds additional [`SetResourceYamlCommand`] options.
pub struct SetResourceYamlOptions {
    pub action: SetResourceYamlAction,
    pub encode: bool,
    pub patch_status: bool,
}

/// Command that apply/patch specified kubernetes resource.
pub struct SetResourceYamlCommand {
    name: String,
    namespace: Namespace,
    yaml: String,
    discovery: Option<(ApiResource, ApiCapabilities)>,
    client: Option<Client>,
    options: SetResourceYamlOptions,
}

impl SetResourceYamlCommand {
    /// Creates new [`SetResourceYamlCommand`] instance.
    pub fn new(
        name: String,
        namespace: Namespace,
        yaml: String,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        options: SetResourceYamlOptions,
    ) -> Self {
        Self {
            name,
            namespace,
            yaml,
            discovery,
            client: Some(client),
            options,
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

        let encode = discovery.0.plural == SECRETS && self.options.encode;
        let patch_status = can_patch_status(&discovery.1) && self.options.patch_status;

        Some(CommandResult::SetResourceYaml(
            self.save_yaml(client, encode, patch_status).await,
        ))
    }

    async fn save_yaml(self, api: Api<DynamicObject>, encode: bool, update_status: bool) -> Result<String, SetResourceYamlError> {
        let mut resource =
            serde_yaml::from_str::<DynamicObject>(&self.yaml).map_err(|e| SetResourceYamlError::SerializationError {
                resource: self.name.clone(),
                source: e,
            })?;

        if encode {
            encode_secret_data(&mut resource);
        }

        let mut status_part = None;
        if let Some(status_val) = resource.data.as_object_mut().and_then(|s| s.remove("status")) {
            status_part = Some(k8s_openapi::serde_json::json!({ "status": status_val }));
        }

        let (patch, patch_params) = match self.options.action {
            SetResourceYamlAction::Apply => (Patch::Apply(&resource), PatchParams::apply(b4n_config::APP_NAME)),
            SetResourceYamlAction::ForceApply => (Patch::Apply(&resource), PatchParams::apply(b4n_config::APP_NAME).force()),
            SetResourceYamlAction::Patch => (Patch::Merge(&resource), PatchParams::default()),
        };

        api.patch(&self.name, &patch_params, &patch)
            .await
            .map_err(|e| SetResourceYamlError::PatchError {
                action: self.options.action,
                resource: self.name.clone(),
                source: e,
            })?;

        if let Some(status) = status_part
            && update_status
        {
            let (patch, patch_params) = match self.options.action {
                SetResourceYamlAction::Apply => (Patch::Apply(&status), PatchParams::apply(b4n_config::APP_NAME)),
                SetResourceYamlAction::ForceApply => (Patch::Apply(&status), PatchParams::apply(b4n_config::APP_NAME).force()),
                SetResourceYamlAction::Patch => (Patch::Merge(&status), PatchParams::default()),
            };

            api.patch_status(&self.name, &patch_params, &patch)
                .await
                .map_err(|e| SetResourceYamlError::PatchError {
                    action: self.options.action,
                    resource: self.name.clone(),
                    source: e,
                })?;
        }

        Ok(self.name)
    }
}
