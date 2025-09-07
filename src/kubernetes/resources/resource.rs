use k8s_openapi::{apimachinery::pkg::apis::meta::v1::Time, serde_json::Value};
use kube::{
    ResourceExt,
    api::{DynamicObject, ObjectMeta},
};
use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    kubernetes::{metrics::Metrics, resources::CrdColumns, utils::get_object_uid, watchers::Statistics},
    ui::{
        colors::TextColors,
        lists::{FilterContext, Filterable, Header, Row},
        theme::Theme,
    },
    utils::{
        logical_expressions::{Expression, ExpressionExt, parse},
        truncate,
    },
};

use super::{ResourceData, ResourceValue, container, get_header_data, get_resource_data};

/// Represents kubernetes resource of any kind.
#[derive(Default)]
pub struct ResourceItem {
    pub uid: String,
    pub name: String,
    pub namespace: Option<String>,
    pub age: Option<String>,
    pub creation_timestamp: Option<Time>,
    pub filter_metadata: Vec<String>,
    pub data: Option<ResourceData>,
}

impl ResourceItem {
    /// Creates light [`ResourceItem`] version just with name.
    pub fn new(name: &str) -> Self {
        Self {
            uid: format!("_{name}_"),
            name: name.to_owned(),
            ..Default::default()
        }
    }

    /// Creates [`ResourceItem`] from kubernetes [`DynamicObject`].
    pub fn from(kind: &str, group: &str, crd: Option<&CrdColumns>, stats: &Statistics, object: DynamicObject) -> Self {
        let data = Some(get_resource_data(kind, group, crd, stats, &object));
        let filter = get_filter_metadata(&object);
        let uid = get_object_uid(&object);
        let creation_timestamp = get_age_time(&object.metadata);

        Self {
            age: get_age_string(&object.metadata),
            name: object.name_any(),
            namespace: object.metadata.namespace,
            uid,
            creation_timestamp,
            filter_metadata: filter,
            data,
        }
    }

    /// Creates [`ResourceItem`] from kubernetes pod container and its metadata.
    pub fn from_container(
        container: &Value,
        status: Option<&Value>,
        pod_metadata: &ObjectMeta,
        metrics: Option<&Metrics>,
        is_init_container: bool,
    ) -> Self {
        let container_name = container["name"].as_str().unwrap_or("unknown").to_owned();
        let uid = pod_metadata
            .uid
            .as_ref()
            .map(|u| format!("{}.{}.{}", u, container_name, if is_init_container { "I" } else { "M" }))
            .unwrap_or_else(|| {
                format!(
                    "{}.{}.{}",
                    pod_metadata.name.as_deref().unwrap_or_default(),
                    container_name,
                    if is_init_container { "I" } else { "M" }
                )
            });

        Self {
            age: get_age_string(pod_metadata),
            name: container_name.clone(),
            namespace: pod_metadata.namespace.clone(),
            uid,
            creation_timestamp: get_age_time(pod_metadata),
            filter_metadata: vec![container_name],
            data: Some(container::data(
                container,
                status,
                metrics,
                is_init_container,
                pod_metadata.deletion_timestamp.is_some(),
            )),
        }
    }

    /// Returns [`Header`] for provided Kubernetes resource kind.
    pub fn header(kind: &str, group: &str, crd: Option<&CrdColumns>, has_metrics: bool) -> Header {
        get_header_data(kind, group, crd, has_metrics)
    }

    /// Returns [`TextColors`] for this kubernetes resource considering `theme` and other data.
    pub fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        if let Some(data) = &self.data {
            data.get_colors(theme, is_active, is_selected)
        } else {
            theme.colors.line.ready.get_specific(is_active, is_selected)
        }
    }

    fn get_extra_values(&self) -> Option<&[ResourceValue]> {
        self.data.as_ref().map(|data| &*data.extra_values)
    }
}

fn get_age_string(metadata: &ObjectMeta) -> Option<String> {
    if metadata.resource_version.is_some() {
        metadata.creation_timestamp.as_ref().map(|t| t.0.timestamp().to_string())
    } else {
        None
    }
}

fn get_age_time(metadata: &ObjectMeta) -> Option<Time> {
    if metadata.resource_version.is_some() {
        metadata.creation_timestamp.clone()
    } else {
        None
    }
}

impl Row for ResourceItem {
    fn uid(&self) -> &str {
        &self.uid
    }

    fn group(&self) -> &str {
        self.namespace.as_deref().unwrap_or_default()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn creation_timestamp(&self) -> Option<&Time> {
        self.creation_timestamp.as_ref()
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        let Some(values) = self.get_extra_values() else {
            return match column {
                0 => Cow::Borrowed(self.namespace.as_deref().unwrap_or("n/a")),
                1 => Cow::Borrowed(self.name.as_str()),
                2 => Cow::Borrowed(self.age.as_deref().unwrap_or("n/a")),
                _ => Cow::Borrowed("n/a"),
            };
        };

        if column == 0 {
            Cow::Borrowed(self.namespace.as_deref().unwrap_or("n/a"))
        } else if column == 1 {
            Cow::Borrowed(self.name.as_str())
        } else if column >= 2 && column <= values.len() + 1 {
            values[column - 2].text()
        } else if column == values.len() + 2 {
            Cow::Borrowed(self.age.as_deref().unwrap_or("n/a"))
        } else {
            Cow::Borrowed("n/a")
        }
    }

    fn column_sort_text(&self, column: usize) -> &str {
        let Some(values) = self.get_extra_values() else {
            return match column {
                0 => self.namespace.as_deref().unwrap_or("n/a"),
                1 => self.name.as_str(),
                2 => self.age.as_deref().unwrap_or("n/a"),
                _ => "n/a",
            };
        };

        if column == 0 {
            self.namespace.as_deref().unwrap_or("n/a")
        } else if column == 1 {
            self.name.as_str()
        } else if column >= 2 && column <= values.len() + 1 {
            values[column - 2].sort_text()
        } else if column == values.len() + 2 {
            self.age.as_deref().unwrap_or("n/a")
        } else {
            "n/a"
        }
    }
}

/// Filtering context for [`ResourceItem`].
pub struct ResourceFilterContext {
    pattern: String,
    extended: Option<Expression>,
}

impl FilterContext for ResourceFilterContext {
    fn restart(&mut self) {
        // Empty implementation.
    }
}

impl Filterable<ResourceFilterContext> for ResourceItem {
    fn get_context(pattern: &str, settings: Option<&str>) -> ResourceFilterContext {
        let expression = if let Some(settings) = settings {
            if settings.contains('e') { parse(pattern).ok() } else { None }
        } else {
            None
        };

        ResourceFilterContext {
            pattern: pattern.to_owned(),
            extended: expression,
        }
    }

    /// Checks if an item match a filter using the provided context.\
    /// Extended filtering is when `e` is provided in settings.\
    /// **Note** that currently it has only a switch for normal/extended filtering.
    fn is_matching(&self, context: &mut ResourceFilterContext) -> bool {
        if let Some(expression) = &context.extended {
            self.filter_metadata.evaluate(expression)
        } else {
            self.name.contains(&context.pattern)
        }
    }
}

fn get_filter_metadata(object: &DynamicObject) -> Vec<String> {
    let mut result = vec![object.name_any().to_ascii_lowercase()];

    if let Some(labels) = object.metadata.labels.as_ref() {
        result.append(&mut flatten_metadata(labels));
    }

    if let Some(annotations) = object.metadata.annotations.as_ref() {
        result.append(&mut flatten_metadata(annotations));
    }

    result
}

fn flatten_metadata(items: &BTreeMap<String, String>) -> Vec<String> {
    items
        .iter()
        .map(|(k, v)| [k.to_ascii_lowercase(), v.to_ascii_lowercase()].join(": "))
        .collect::<Vec<String>>()
}
