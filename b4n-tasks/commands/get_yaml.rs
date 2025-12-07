use b4n_kube::{Kind, Namespace, SECRETS};
use base64::{DecodeError, Engine, engine};
use k8s_openapi::serde_json::Value;
use kube::Client;
use kube::api::{ApiResource, DynamicObject};
use kube::discovery::{ApiCapabilities, verbs};
use ratatui::style::Style;
use tokio::sync::mpsc::UnboundedSender;

use crate::commands::GetNewResourceYamlResult;
use crate::{HighlightRequest, HighlightResourceError, commands::CommandResult, highlight_resource};

/// Possible errors from fetching or styling resource's YAML.
#[derive(thiserror::Error, Debug)]
pub enum ResourceYamlError {
    /// Get is not supported for the specified resource.
    #[error("get is not supported for the specified resource")]
    GetNotSupported,

    /// Unable to retrieve the resource's YAML.
    #[error("unable to retrieve the resource's YAML")]
    GetYamlError(#[from] kube::Error),

    /// Cannot decode resource's data.
    #[error("cannot decode resource's data")]
    SecretDecodeError(#[from] DecodeError),

    /// Cannot highlight provided data.
    #[error("cannot highlight provided data")]
    HighlighterError(#[from] HighlightResourceError),
}

/// Result for the [`GetResourceYamlCommand`] command.
pub struct ResourceYamlResult {
    pub name: String,
    pub namespace: Namespace,
    pub kind: Kind,
    pub yaml: Vec<String>,
    pub styled: Vec<Vec<(Style, String)>>,
    pub is_decoded: bool,
    pub is_editable: bool,
}

impl From<GetNewResourceYamlResult> for ResourceYamlResult {
    fn from(value: GetNewResourceYamlResult) -> Self {
        Self {
            name: String::new(),
            namespace: value.namespace,
            kind: value.kind,
            yaml: value.yaml,
            styled: value.styled,
            is_decoded: false,
            is_editable: true,
        }
    }
}

/// Command that gets a specified resource from the kubernetes API and then styles it.
pub struct GetResourceYamlCommand {
    name: String,
    namespace: Namespace,
    kind: Kind,
    discovery: Option<(ApiResource, ApiCapabilities)>,
    client: Option<Client>,
    highlighter: UnboundedSender<HighlightRequest>,
    decode: bool,
    sanitize: bool,
}

impl GetResourceYamlCommand {
    /// Creates new [`GetResourceYamlCommand`] instance.
    pub fn new(
        name: String,
        namespace: Namespace,
        kind: Kind,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        highlighter: UnboundedSender<HighlightRequest>,
    ) -> Self {
        Self {
            name,
            namespace,
            kind,
            discovery,
            client: Some(client),
            highlighter,
            decode: false,
            sanitize: false,
        }
    }

    /// Creates new [`GetResourceYamlCommand`] instance that will try to decode secret's data.
    pub fn decoded(
        name: String,
        namespace: Namespace,
        kind: Kind,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        highlighter: UnboundedSender<HighlightRequest>,
    ) -> Self {
        let decode = kind.as_str() == SECRETS;
        let mut command = GetResourceYamlCommand::new(name, namespace, kind, discovery, client, highlighter);
        command.decode = decode;
        command
    }

    /// Creates new [`GetResourceYamlCommand`] instance that will decode and sanitize fetched resource.
    pub fn sanitized(
        name: String,
        namespace: Namespace,
        kind: Kind,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        highlighter: UnboundedSender<HighlightRequest>,
    ) -> Self {
        let decode = kind.name() == SECRETS;
        let mut command = GetResourceYamlCommand::new(name, namespace, kind, discovery, client, highlighter);
        command.sanitize = true;
        command.decode = decode;
        command
    }

    /// Returns YAML representation of the kubernetes resource.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::GET) {
            return Some(CommandResult::GetResourceYaml(Err(ResourceYamlError::GetNotSupported)));
        }

        let client = b4n_kube::client::get_dynamic_api(
            &discovery.0,
            &discovery.1,
            self.client.take().expect("kubernetes client should be present"),
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        match client.get(&self.name).await {
            Ok(resource) => Some(CommandResult::GetResourceYaml(
                self.style_resource(resource, &discovery.1).await,
            )),
            Err(err) => Some(CommandResult::GetResourceYaml(Err(ResourceYamlError::GetYamlError(err)))),
        }
    }

    async fn style_resource(
        self,
        mut resource: DynamicObject,
        cap: &ApiCapabilities,
    ) -> Result<ResourceYamlResult, ResourceYamlError> {
        if self.decode {
            decode_secret_data(&mut resource)?;
        }

        if self.sanitize {
            sanitize(&mut resource);
        }

        match highlight_resource(&self.highlighter, resource).await {
            Ok(response) => Ok(ResourceYamlResult {
                name: if self.sanitize { String::new() } else { self.name },
                namespace: self.namespace,
                kind: self.kind,
                yaml: response.plain,
                styled: response.styled,
                is_decoded: self.decode,
                is_editable: cap.supports_operation(verbs::PATCH),
            }),
            Err(err) => Err(err.into()),
        }
    }
}

fn decode_secret_data(resource: &mut DynamicObject) -> Result<(), DecodeError> {
    if resource.data.get("data").is_some_and(Value::is_object) {
        let engine = engine::general_purpose::STANDARD;
        for mut data in resource.data["data"].as_object_mut().unwrap().iter_mut() {
            if let Value::String(data) = &mut data.1 {
                let decoded_bytes = engine.decode(&data)?;
                *data = String::from_utf8_lossy(&decoded_bytes).to_string();
            }
        }
    }

    Ok(())
}

fn sanitize(resource: &mut DynamicObject) {
    resource.metadata.creation_timestamp = None;
    resource.metadata.deletion_grace_period_seconds = None;
    resource.metadata.deletion_timestamp = None;
    resource.metadata.generate_name = None;
    resource.metadata.generation = None;
    resource.metadata.managed_fields = None;
    resource.metadata.name = Some(String::new());
    resource.metadata.owner_references = None;
    resource.metadata.resource_version = None;
    resource.metadata.self_link = None;
    resource.metadata.uid = None;
    if let Value::Object(map) = &mut resource.data {
        map.remove("status");
    }
}
