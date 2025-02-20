use kube::{
    Client, ResourceExt,
    api::{ApiResource, DynamicObject},
    discovery::{ApiCapabilities, verbs},
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
    pub kind_plural: String,
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

        let plural = discovery.0.plural.clone();
        let client = kubernetes::client::get_dynamic_api(
            discovery.0,
            discovery.1,
            self.client,
            self.namespace.as_option(),
            self.namespace.is_all(),
        );

        match client.get(&self.name).await {
            Ok(resource) => Some(CommandResult::ResourceYaml(
                style_resources_yaml(resource, self.syntax, self.name, self.namespace, plural).await,
            )),
            Err(err) => Some(CommandResult::ResourceYaml(Err(ResourceYamlError::GetYamlError(err)))),
        }
    }
}

async fn style_resources_yaml(
    mut resource: DynamicObject,
    data: SyntaxData,
    name: String,
    namespace: Namespace,
    kind_plural: String,
) -> Result<ResourceYamlResult, ResourceYamlError> {
    tokio::task::spawn_blocking(move || {
        resource.managed_fields_mut().clear();
        let mut yaml = serde_yaml::to_string(&resource)?;

        if let Some(index) = yaml.find("\n  managedFields: []\n") {
            yaml.replace_range(index + 1..index + 21, "");
        }

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
            kind_plural,
            yaml: lines,
            styled,
        })
    })
    .await?
}

fn convert_style(style: syntect::highlighting::Style) -> Style {
    Style::default()
        .fg(from_syntect_color(style.foreground))
        .bg(from_syntect_color(style.background))
}
