use b4n_kube::utils::can_patch_status;
use b4n_kube::{Kind, Namespace, Scope};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use k8s_openapi::serde_json::{self, Map, Value};
use kube::{Api, Client, api::ApiResource, discovery::ApiCapabilities};
use ratatui::style::Style;
use tokio::sync::mpsc::UnboundedSender;

use crate::{HighlightRequest, HighlightResourceError, commands::CommandResult, highlight_yaml};

/// Errors that may occur while fetching or styling a resource's YAML template.
#[derive(thiserror::Error, Debug)]
pub enum GetNewResourceYamlError {
    /// Cannot fetch `OpenAPI` resource schemas.
    #[error("cannot fetch OpenAPI resource schemas")]
    OpenApiRequest(#[from] kube::Error),

    /// Failed to serialize YAML schemas.
    #[error("failed to serialize YAML schemas")]
    YamlSerializationError(#[from] serde_yaml::Error),

    /// Failed to serialize YAML schemas.
    #[error("failed to serialize YAML schemas")]
    JsonSerializationError(#[from] serde_json::Error),

    /// Cannot find schema definition for the resource.
    #[error("cannot find schema definition for the resource")]
    SchemaNotFound,

    /// Cannot highlight provided data.
    #[error("cannot highlight provided data")]
    HighlighterError(#[from] HighlightResourceError),
}

/// Result of generating YAML for a resource.
#[derive(Debug)]
pub struct GetNewResourceYamlResult {
    pub namespace: Namespace,
    pub kind: Kind,
    pub singular: String,
    pub yaml: Vec<String>,
    pub styled: Vec<Vec<(Style, String)>>,
    pub can_patch_status: bool,
}

/// Command for generating a YAML template for a Kubernetes resource.
pub struct GetNewResourceYamlCommand {
    namespace: Namespace,
    kind: Kind,
    discovery: Option<(ApiResource, ApiCapabilities)>,
    client: Option<Client>,
    highlighter: UnboundedSender<HighlightRequest>,
    required_only: bool,
}

impl GetNewResourceYamlCommand {
    /// Creates a new command instance.
    pub fn new(
        namespace: Namespace,
        kind: Kind,
        discovery: Option<(ApiResource, ApiCapabilities)>,
        client: Client,
        highlighter: UnboundedSender<HighlightRequest>,
        required_only: bool,
    ) -> Self {
        Self {
            namespace,
            kind,
            discovery,
            client: Some(client),
            highlighter,
            required_only,
        }
    }

    /// Executes the command and generates YAML template for the resource.
    pub async fn execute(self) -> Option<CommandResult> {
        let client = self.client.as_ref()?;
        let (res, cap) = self.discovery.as_ref()?;

        let result = async {
            let (root, schema) = get_resource_schema(client, res).await?;
            let yaml_val = build_resource(res, cap, self.namespace.clone(), &root, &schema, self.required_only);
            let yaml_str = serde_yaml::to_string(&yaml_val)?;
            self.style_yaml(yaml_str, res, cap).await
        }
        .await;

        Some(CommandResult::GetNewResourceYaml(result))
    }

    async fn style_yaml(
        &self,
        yaml: String,
        res: &ApiResource,
        cap: &ApiCapabilities,
    ) -> Result<GetNewResourceYamlResult, GetNewResourceYamlError> {
        match highlight_yaml(&self.highlighter, yaml).await {
            Ok(resp) => Ok(GetNewResourceYamlResult {
                namespace: self.namespace.clone(),
                kind: self.kind.clone(),
                singular: res.kind.clone(),
                yaml: resp.plain,
                styled: resp.styled,
                can_patch_status: can_patch_status(cap),
            }),
            Err(e) => Err(e.into()),
        }
    }
}

async fn get_resource_schema(client: &Client, resource: &ApiResource) -> Result<(Value, Value), GetNewResourceYamlError> {
    if let Ok((root, schema)) = get_builtin_schema(client, resource).await {
        return Ok((root, schema));
    }

    if let Ok(schema) = get_crd_schema(client, resource).await {
        return Ok((Value::Null, schema));
    }

    Err(GetNewResourceYamlError::SchemaNotFound)
}

async fn get_builtin_schema(client: &Client, resource: &ApiResource) -> Result<(Value, Value), GetNewResourceYamlError> {
    let path = if resource.group.is_empty() {
        format!("/openapi/v3/api/{}", resource.version)
    } else {
        format!("/openapi/v3/apis/{}/{}", resource.group, resource.version)
    };

    let req = http::Request::get(&path)
        .body(Vec::default())
        .expect("failed to build OpenAPI request");

    let text = client.request_text(req).await?;
    let mut doc: Value = serde_json::from_str(&text)?;

    let schemas = doc
        .get_mut("components")
        .and_then(|c| c.get_mut("schemas"))
        .map(Value::take)
        .ok_or(GetNewResourceYamlError::SchemaNotFound)?;

    let key = format!(
        "io.k8s.api.{}.{}.{}",
        if resource.group.is_empty() { "core" } else { &resource.group },
        &resource.version,
        &resource.kind,
    );

    let schema = schemas
        .as_object()
        .and_then(|map| map.get(&key))
        .cloned()
        .ok_or(GetNewResourceYamlError::SchemaNotFound)?;

    Ok((schemas, schema))
}

async fn get_crd_schema(client: &Client, res: &ApiResource) -> Result<Value, GetNewResourceYamlError> {
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    let crd_name = format!("{}.{}", res.plural, res.group);
    let crd = crds.get(&crd_name).await?;

    for version in &crd.spec.versions {
        if version.name == res.version
            && let Some(schema) = version.schema.as_ref().and_then(|s| s.open_api_v3_schema.as_ref())
        {
            return Ok(serde_json::to_value(schema)?);
        }
    }

    Err(GetNewResourceYamlError::SchemaNotFound)
}

fn build_resource(
    res: &ApiResource,
    cap: &ApiCapabilities,
    namespace: Namespace,
    root: &Value,
    schema: &Value,
    required_only: bool,
) -> Value {
    let mut new_resource = Map::new();

    let api_version = if res.group.is_empty() {
        res.version.clone()
    } else {
        format!("{}/{}", res.group, res.version)
    };
    new_resource.insert("apiVersion".into(), Value::String(api_version));
    new_resource.insert("kind".into(), Value::String(res.kind.clone()));

    let mut meta = Map::new();
    meta.insert("name".into(), Value::String(String::new()));

    if cap.scope == Scope::Namespaced {
        meta.insert(
            "namespace".into(),
            Value::String(if namespace.is_all() { String::new() } else { namespace.into() }),
        );
    }

    new_resource.insert("metadata".into(), Value::Object(meta));

    if let Some(spec_schema) = schema.get("properties").and_then(|p| p.get("spec")) {
        new_resource.insert("spec".into(), template_from_schema(root, spec_schema, required_only));
    }

    if can_patch_status(cap)
        && let Some(spec_schema) = schema.get("properties").and_then(|p| p.get("status"))
    {
        new_resource.insert("status".into(), template_from_schema(root, spec_schema, required_only));
    }

    Value::Object(new_resource)
}

fn template_from_schema(root: &Value, schema: &Value, required_only: bool) -> Value {
    let resolved = resolve_schema(root, schema);

    let required: Vec<String> = resolved
        .get("required")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(ToString::to_string)).collect())
        .unwrap_or_default();

    match resolved.get("type").and_then(|v| v.as_str()) {
        Some("object") => {
            let mut map = Map::new();

            if let Some(props) = resolved.get("properties").and_then(|p| p.as_object()) {
                for (k, prop_schema) in props {
                    if !required_only || required.contains(k) {
                        map.insert(k.clone(), template_from_schema(root, prop_schema, required_only));
                    }
                }
            }

            Value::Object(map)
        },
        Some("array") => Value::Array(Vec::new()),
        Some("string") => Value::String(String::new()),
        Some("integer" | "number") => Value::Number(0.into()),
        Some("boolean") => Value::Bool(false),

        _ => Value::Null,
    }
}

fn resolve_schema<'a>(root: &'a Value, schema: &'a Value) -> Value {
    if let Some(r) = schema.get("$ref").and_then(|v| v.as_str())
        && let Some(stripped) = r.strip_prefix("#/components/schemas/")
        && let Some(obj) = root.as_object().and_then(|s| s.get(stripped))
    {
        return obj.clone();
    }

    if let Some(arr) = schema.get("allOf").and_then(|a| a.as_array()) {
        let mut merged = serde_json::json!({});
        for entry in arr {
            let resolved = resolve_schema(root, entry);
            merge_objects(&mut merged, &resolved);
        }

        return merged;
    }

    schema.clone()
}

fn merge_objects(target: &mut Value, src: &Value) {
    if let (Some(target_map), Some(src_map)) = (target.as_object_mut(), src.as_object()) {
        for (k, v) in src_map {
            target_map.insert(k.clone(), v.clone());
        }
    }
}
