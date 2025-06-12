use base64::{DecodeError, Engine, engine};
use k8s_openapi::serde_json::Value;
use kube::{
    Client,
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, verbs},
};
use ratatui::style::Style;
use syntect::easy::HighlightLines;

use crate::{
    core::SyntaxData,
    kubernetes::{self, Kind, Namespace, resources::SECRETS, utils},
    ui::colors::from_syntect_color,
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

    /// YAML syntax definition not found.
    #[error("YAML syntax definition not found")]
    SyntaxNotFound,

    /// Cannot highlight YAML syntax.
    #[error("cannot highlight YAML syntax")]
    SyntaxHighlightingError(#[from] syntect::Error),

    /// Syntax highlighting task failed.
    #[error("syntax highlighting task failed")]
    HighlightingTaskError(#[from] tokio::task::JoinError),

    /// Cannot decode resource's data.
    #[error("cannot decode resource's data")]
    SecretDecodeError(#[from] DecodeError),
}

/// Result for the [`GetResourceYamlCommand`] command.
pub struct ResourceYamlResult {
    pub name: String,
    pub namespace: Namespace,
    pub kind: Kind,
    pub yaml: Vec<String>,
    pub styled: Vec<Vec<(Style, String)>>,
    pub is_decoded: bool,
}

/// Command that gets a specified resource from the kubernetes API and then styles it.
pub struct GetResourceYamlCommand {
    name: String,
    namespace: Namespace,
    kind: Kind,
    discovery: Option<(ApiResource, ApiCapabilities)>,
    client: Client,
    syntax: SyntaxData,
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
        syntax: SyntaxData,
    ) -> Self {
        Self {
            name,
            namespace,
            kind,
            discovery,
            client,
            syntax,
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
        syntax: SyntaxData,
    ) -> Self {
        let decode = kind.as_str() == SECRETS;
        let mut command = GetResourceYamlCommand::new(name, namespace, kind, discovery, client, syntax);
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
            self.client,
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        match client.get(&self.name).await {
            Ok(resource) => Some(CommandResult::ResourceYaml(
                style_resource(resource, self.syntax, self.name, self.namespace, self.kind, self.decode).await,
            )),
            Err(err) => Some(CommandResult::ResourceYaml(Err(ResourceYamlError::GetYamlError(err)))),
        }
    }
}

async fn style_resource(
    mut resource: DynamicObject,
    data: SyntaxData,
    name: String,
    namespace: Namespace,
    kind: Kind,
    decode: bool,
) -> Result<ResourceYamlResult, ResourceYamlError> {
    tokio::task::spawn_blocking(move || {
        if decode {
            decode_secret_data(&mut resource)?;
        }

        let yaml = utils::serialize_resource(&mut resource)?;
        let lines = yaml.split_inclusive('\n').map(String::from).collect::<Vec<_>>();
        let syntax = data
            .syntax_set
            .find_syntax_by_extension("yaml")
            .ok_or(ResourceYamlError::SyntaxNotFound)?;

        let mut h = HighlightLines::new(syntax, &data.yaml_theme);

        let styled = lines
            .iter()
            .map(|line| {
                Ok(h.highlight_line(line, &data.syntax_set)?
                    .into_iter()
                    .map(|segment| (convert_style(segment.0), segment.1.to_owned()))
                    .collect::<Vec<_>>())
            })
            .collect::<Result<Vec<_>, syntect::Error>>()?;

        Ok(ResourceYamlResult {
            name,
            namespace,
            kind,
            yaml: lines,
            styled,
            is_decoded: decode,
        })
    })
    .await?
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

fn convert_style(style: syntect::highlighting::Style) -> Style {
    Style::default()
        .fg(from_syntect_color(style.foreground))
        .bg(from_syntect_color(style.background))
}
