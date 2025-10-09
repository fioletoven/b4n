use base64::{DecodeError, Engine, engine};
use k8s_openapi::serde_json::Value;
use kube::{
    Client,
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, verbs},
};
use ratatui::style::Style;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    core::highlighter::{HighlightError, HighlightRequest},
    kubernetes::{self, Kind, Namespace, resources::SECRETS, utils},
};

use super::CommandResult;

/// Possible errors from fetching or styling resource's YAML.
#[derive(thiserror::Error, Debug)]
pub enum ResourceYamlError {
    /// Get is not supported for the specified resource.
    #[error("get is not supported for the specified resource")]
    GetNotSupported,

    /// Unable to retrieve the resource's YAML.
    #[error("unable to retrieve the resource's YAML")]
    GetYamlError(#[from] kube::Error),

    /// Cannot serialize resource's YAML.
    #[error("cannot serialize resource's YAML")]
    SerializationError(#[from] serde_yaml::Error),

    /// Cannot send syntax higlight request to the highlighter thread.
    #[error("cannot send syntax higlight request")]
    CannotSendRequest(#[from] tokio::sync::mpsc::error::SendError<HighlightRequest>),

    /// Cannot send syntax higlight request to the highlighter thread.
    #[error("cannot send syntax higlight request")]
    CannotRecvResponse(#[from] tokio::sync::oneshot::error::RecvError),

    /// Cannot decode resource's data.
    #[error("cannot decode resource's data")]
    SecretDecodeError(#[from] DecodeError),

    /// Cannot highlight provided data.
    #[error("cannot highlight provided data")]
    HighlighterError(#[from] HighlightError),
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

/// Command that gets a specified resource from the kubernetes API and then styles it.
pub struct GetResourceYamlCommand {
    name: String,
    namespace: Namespace,
    kind: Kind,
    discovery: Option<(ApiResource, ApiCapabilities)>,
    client: Option<Client>,
    highlighter: UnboundedSender<HighlightRequest>,
    decode: bool,
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
        }
    }

    /// Creates new [`GetResourceYamlCommand`] instance that will try to decode secret's data.
    pub fn decode(
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

    /// Returns YAML representation of the kubernetes resource.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::GET) {
            return Some(CommandResult::ResourceYaml(Err(ResourceYamlError::GetNotSupported)));
        }

        let client = kubernetes::client::get_dynamic_api(
            &discovery.0,
            &discovery.1,
            self.client.take().expect("kubernetes client should be present"),
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        match client.get(&self.name).await {
            Ok(resource) => Some(CommandResult::ResourceYaml(self.style_resource(resource, &discovery.1).await)),
            Err(err) => Some(CommandResult::ResourceYaml(Err(ResourceYamlError::GetYamlError(err)))),
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

        let yaml = utils::serialize_resource(&mut resource)?;
        let mut plain = yaml.split('\n').map(String::from).collect::<Vec<_>>();
        if yaml.ends_with('\n') {
            plain.pop();
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.highlighter.send(HighlightRequest::Full {
            lines: plain,
            response: tx,
        })?;

        match rx.await? {
            Ok(response) => Ok(ResourceYamlResult {
                name: self.name,
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
