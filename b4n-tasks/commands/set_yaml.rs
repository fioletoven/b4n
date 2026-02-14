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

impl SetResourceYamlAction {
    /// Returns [`SetResourceYamlAction`] variant.
    pub fn from(is_apply: bool, is_forced: bool) -> Self {
        match (is_apply, is_forced) {
            (true, true) => SetResourceYamlAction::ForceApply,
            (true, false) => SetResourceYamlAction::Apply,
            _ => SetResourceYamlAction::Patch,
        }
    }
}

/// Holds additional [`SetResourceYamlCommand`] options.
pub struct SetResourceYamlOptions {
    pub action: SetResourceYamlAction,
    pub encode: bool,
    pub patch_status: bool,
    pub ignore_version: bool,
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
        let ignore_version = self.options.ignore_version;

        Some(CommandResult::SetResourceYaml(
            self.save_yaml(client, encode, patch_status, ignore_version).await,
        ))
    }

    async fn save_yaml(
        self,
        api: Api<DynamicObject>,
        encode: bool,
        update_status: bool,
        ignore_version: bool,
    ) -> Result<String, SetResourceYamlError> {
        let mut resource = serde_yaml::from_str::<k8s_openapi::serde_json::Value>(&self.yaml).map_err(|e| {
            SetResourceYamlError::SerializationError {
                resource: self.name.clone(),
                source: e,
            }
        })?;

        if encode {
            encode_secret_data(resource.get_mut("data"));
        }

        if ignore_version && let Some(rv) = resource["metadata"].get_mut("resourceVersion") {
            *rv = k8s_openapi::serde_json::Value::Null;
        }

        let status_part = resource
            .as_object_mut()
            .and_then(|o| o.remove("status"))
            .map(|s| k8s_openapi::serde_json::json!({ "status": s }));

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
