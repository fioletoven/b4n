use kube::{
    api::{ApiResource, DynamicObject},
    discovery::{verbs, ApiCapabilities},
    Client, ResourceExt,
};
use ratatui::style::Style;
use syntect::easy::HighlightLines;

use crate::{
    app::SyntaxData,
    kubernetes::{self, Namespace},
    ui::colors::from_syntect_color,
};

use super::CommandResult;

/// Possible errors when fetching resources YAML.
#[derive(thiserror::Error, Debug)]
pub enum ResourceYamlError {
    /// Get is not supported for the specified resource.
    #[error("get is not supported for the specified resource")]
    GetNotSupported,

    /// Cannot get resources YAML.
    #[error("cannot get resources YAML")]
    GetYamlError(#[from] kube::Error),

    /// Cannot serialize resources YAML.
    #[error("cannot serialize resources YAML")]
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
}

/// Result for the [`GetYamlCommand`].
pub struct ResourceYamlResult {
    pub name: String,
    pub namespace: Namespace,
    pub yaml: Vec<String>,
    pub styled: Vec<Vec<(Style, String)>>,
}

/// Command that returns YAML representation of a specified kubernetes resource.
pub struct GetResourceYamlCommand {
    pub name: String,
    pub namespace: Namespace,
    pub discovery: Option<(ApiResource, ApiCapabilities)>,
    pub client: Client,
    pub syntax: SyntaxData,
}

impl GetResourceYamlCommand {
    /// Creates new [`GetYamlCommand`] instance.
    pub fn new(
        name: String,
        namespace: Namespace,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        syntax: SyntaxData,
    ) -> Self {
        Self {
            name,
            namespace,
            discovery,
            client,
            syntax,
        }
    }

    /// Returns YAML representation of the kubernetes resource.
    pub async fn execute(mut self) -> Option<CommandResult> {
        let discovery = self.discovery.take()?;
        if !discovery.1.supports_operation(verbs::GET) {
            return Some(CommandResult::ResourceYaml(Err(ResourceYamlError::GetNotSupported)));
        }

        let client = kubernetes::client::get_dynamic_api(
            discovery.0.clone(),
            discovery.1.clone(),
            self.client.clone(),
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        match client.get(&self.name).await {
            Ok(resource) => Some(CommandResult::ResourceYaml(self.style_resources_yaml(resource).await)),
            Err(err) => Some(CommandResult::ResourceYaml(Err(ResourceYamlError::GetYamlError(err)))),
        }
    }

    async fn style_resources_yaml(self, mut resource: DynamicObject) -> Result<ResourceYamlResult, ResourceYamlError> {
        tokio::task::spawn_blocking(move || {
            resource.managed_fields_mut().clear();
            let mut resource = serde_yaml::to_string(&resource)?;

            if let Some(index) = resource.find("\n  managedFields: []\n") {
                resource.replace_range(index + 1..index + 21, "");
            }

            let lines = resource.split_inclusive('\n').map(String::from).collect::<Vec<_>>();
            let syntax = self
                .syntax
                .syntax_set
                .find_syntax_by_extension("yaml")
                .ok_or(ResourceYamlError::SyntaxNotFound)?;
            let mut h = HighlightLines::new(syntax, &self.syntax.yaml_theme);

            let styled = lines
                .iter()
                .map(|line| {
                    Ok(h.highlight_line(line, &self.syntax.syntax_set)?
                        .into_iter()
                        .map(|segment| (convert_style(segment.0), segment.1.to_owned()))
                        .collect::<Vec<_>>())
                })
                .collect::<Result<Vec<_>, syntect::Error>>()?;

            Ok(ResourceYamlResult {
                name: self.name,
                namespace: self.namespace,
                yaml: lines,
                styled,
            })
        })
        .await?
    }
}

fn convert_style(style: syntect::highlighting::Style) -> Style {
    Style::default()
        .fg(from_syntect_color(style.foreground))
        .bg(from_syntect_color(style.background))
}
