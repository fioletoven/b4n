use kube::{
    Api, Client,
    api::{ApiResource, DynamicObject, Patch, PatchParams},
    discovery::{ApiCapabilities, verbs},
};

use crate::{
    core::{APP_NAME, commands::CommandResult},
    kubernetes::{self, Namespace},
};

/// Possible errors from applying or patching resource's YAML.
#[derive(thiserror::Error, Debug)]
pub enum SetResourceYamlError {
    /// Patch is not supported for the specified resource.
    #[error("patch is not supported for the specified resource")]
    PatchNotSupported,

    /// Cannot de-serialize resource's YAML.
    #[error("cannot de-serialize resource's YAML")]
    SerializationError(#[from] serde_yaml::Error),

    /// Unable to save the resource's YAML.
    #[error("unable to save the resource's YAML")]
    SetYamlError(#[from] kube::Error),
}

pub enum SetResourceYamlAction {
    Apply,
    ForceApply,
    Patch,
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

        let client = kubernetes::client::get_dynamic_api(
            &discovery.0,
            &discovery.1,
            self.client.take().expect("kubernetes client should be present"),
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        Some(CommandResult::SetResourceYaml(self.save_yaml(client).await))
    }

    async fn save_yaml(self, api: Api<DynamicObject>) -> Result<String, SetResourceYamlError> {
        let yaml = serde_yaml::from_str::<DynamicObject>(&self.yaml)?;

        let (patch, patch_params) = match self.action {
            SetResourceYamlAction::Apply => (Patch::Apply(&yaml), PatchParams::apply(APP_NAME)),
            SetResourceYamlAction::ForceApply => (Patch::Apply(&yaml), PatchParams::apply(APP_NAME).force()),
            SetResourceYamlAction::Patch => (Patch::Merge(&yaml), PatchParams::default()),
        };

        api.patch(&self.name, &patch_params, &patch).await?;

        Ok(self.name)
    }
}
