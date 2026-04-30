use b4n_common::expr::{Expression, ExpressionExt, SelectiveMap, parse};
use b4n_common::truncate;
use b4n_config::themes::{TextColors, Theme};
use b4n_kube::stats::{Metrics, Statistics};
use b4n_kube::{Kind, Namespace, PV};
use b4n_kube::{crds::CrdColumns, utils::get_object_uid};
use b4n_list::{FilterContext, Filterable, Row};
use b4n_tui::table::Header;
use k8s_openapi::jiff::Timestamp;
use k8s_openapi::serde_json::Value;
use kube::api::{DynamicObject, ObjectMeta};
use std::{borrow::Cow, collections::BTreeMap};

use crate::kube::resources::{
    ResourceData, ResourceValue, condition, container, get_header_data, get_resource_data, get_resource_name,
};

#[cfg(test)]
#[path = "./resource.tests.rs"]
mod resource_tests;

/// Represents involved object of the resource.
pub struct InvolvedObject {
    pub kind: Kind,
    pub namespace: Namespace,
    pub name: String,
}

/// What kind of columns should be displayed on the screen.
#[derive(Clone, Copy)]
pub enum ColumnsLayout {
    /// Normal resources view.
    General,

    /// One object view.
    Individual,

    /// Describe view.
    Compact,
}

/// Represents kubernetes resource of any kind.
#[derive(Default)]
pub struct ResourceItem {
    pub uid: String,
    pub name: String,
    pub namespace: Option<String>,
    pub age: Option<String>,
    pub data: Option<ResourceData>,
    pub involved_object: Option<InvolvedObject>,
    pub is_cached: bool,
    creation_timestamp: Option<Timestamp>,
    filter_metadata: SelectiveMap,
    ignore_filters: bool,
}

impl ResourceItem {
    /// Creates light [`ResourceItem`] version just with name.
    pub fn new(name: &str, ignore_filters: bool) -> Self {
        Self {
            uid: format!("_{name}_"),
            name: name.to_owned(),
            ignore_filters,
            ..Default::default()
        }
    }

    /// Creates [`ResourceItem`] from kubernetes [`DynamicObject`].
    pub fn from(
        kind: &str,
        group: &str,
        crd: Option<&CrdColumns>,
        stats: &Statistics,
        object: DynamicObject,
        columns_layout: ColumnsLayout,
    ) -> Self {
        let data = Some(get_resource_data(kind, group, crd, stats, &object, columns_layout));
        let filter = get_filter_metadata(kind, group, &object.metadata);
        let uid = get_object_uid(&object);
        let creation_timestamp = get_age_time(&object.metadata);
        let involved_object = get_involved_object(kind, &object);

        Self {
            age: get_age_string(creation_timestamp),
            name: get_resource_name(kind, group, &object, columns_layout),
            namespace: object.metadata.namespace,
            uid,
            data,
            involved_object,
            creation_timestamp,
            filter_metadata: filter,
            ..Default::default()
        }
    }

    /// Creates [`ResourceItem`] from kubernetes pod container and its metadata.
    pub fn from_container(
        container: &Value,
        status: Option<&Value>,
        pod_metadata: &ObjectMeta,
        metrics: Option<Metrics>,
        is_init_container: bool,
    ) -> Self {
        let container_name = container["name"].as_str().unwrap_or("unknown").to_owned();
        let creation_timestamp = get_start_time(status, pod_metadata);
        let id_prefix = pod_metadata
            .uid
            .as_deref()
            .or(pod_metadata.name.as_deref())
            .unwrap_or_default();
        let uid = format!(
            "{}.{}.{}",
            id_prefix,
            container_name,
            if is_init_container { "I" } else { "M" }
        );
        let mut filter = get_filter_metadata("Container", "", pod_metadata);
        filter.insert("n", vec![container_name.to_ascii_lowercase()]);

        Self {
            age: get_age_string(creation_timestamp),
            name: container_name,
            namespace: pod_metadata.namespace.clone(),
            uid,
            data: Some(container::data(
                container,
                status,
                metrics,
                is_init_container,
                pod_metadata.deletion_timestamp.is_some(),
            )),
            creation_timestamp,
            filter_metadata: filter,
            ..Default::default()
        }
    }

    /// Creates [`ResourceItem`] from kubernetes resource status condition.
    pub fn from_status_condition(status_condition: &Value) -> Self {
        let creation_timestamp = get_transition_time(status_condition);
        let condition_type = status_condition["type"].as_str().unwrap_or("unknown").to_owned();
        let uid = format!("_{condition_type}_");

        Self {
            age: get_age_string(creation_timestamp),
            name: condition_type,
            uid,
            data: Some(condition::data(status_condition)),
            creation_timestamp,
            ..Default::default()
        }
    }

    /// Updates specified data column text.
    pub fn set_data_text(&mut self, idx: usize, text: impl Into<String>) {
        if let Some(data) = &mut self.data
            && let Some(value) = data.extra_values.get_mut(idx)
        {
            value.set_raw_text(text.into());
        }
    }

    /// Returns [`Header`] for provided Kubernetes resource kind.
    pub fn header(kind: &str, group: &str, crd: Option<&CrdColumns>, has_metrics: bool, columns_layout: ColumnsLayout) -> Header {
        get_header_data(kind, group, crd, has_metrics, columns_layout)
    }

    /// Returns [`TextColors`] for this kubernetes resource considering `theme` and other data.
    pub fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        if let Some(data) = &self.data {
            if self.is_cached {
                data.get_colors(&theme.colors.line_cached, is_active, is_selected)
            } else {
                data.get_colors(&theme.colors.line, is_active, is_selected)
            }
        } else if self.is_cached {
            theme.colors.line_cached.ready.get_specific(is_active, is_selected)
        } else {
            theme.colors.line.ready.get_specific(is_active, is_selected)
        }
    }

    fn get_extra_values(&self) -> Option<&[ResourceValue]> {
        self.data.as_ref().map(|data| &*data.extra_values)
    }
}

fn get_age_time(metadata: &ObjectMeta) -> Option<Timestamp> {
    if metadata.resource_version.is_some() {
        metadata.creation_timestamp.as_ref().map(|t| t.0)
    } else {
        None
    }
}

fn get_start_time(status: Option<&Value>, metadata: &ObjectMeta) -> Option<Timestamp> {
    if let Some(status) = status {
        if let Some(started_at) = status["state"]["running"]["startedAt"].as_str() {
            return started_at.parse().ok();
        }

        if let Some(started_at) = status["state"]["terminated"]["startedAt"].as_str() {
            return started_at.parse().ok();
        }

        if let Some(started_at) = status["lastState"]["running"]["startedAt"].as_str() {
            return started_at.parse().ok();
        }

        if let Some(started_at) = status["lastState"]["terminated"]["startedAt"].as_str() {
            return started_at.parse().ok();
        }
    }

    get_age_time(metadata)
}

fn get_transition_time(condition: &Value) -> Option<Timestamp> {
    if let Some(transition_time) = condition["lastTransitionTime"].as_str() {
        transition_time.parse().ok()
    } else {
        None
    }
}

fn get_age_string(timestamp: Option<Timestamp>) -> Option<String> {
    timestamp.map(|t| t.as_millisecond().to_string())
}

fn get_involved_object(kind: &str, object: &DynamicObject) -> Option<InvolvedObject> {
    if let Some(object) = object.data.get("involvedObject") {
        return get_involved_object_from_ref(object);
    }

    if let Some(object) = object.data["spec"].get("claimRef") {
        return get_involved_object_from_ref(object);
    }

    if kind == "PersistentVolumeClaim"
        && let Some(name) = object.data["spec"]["volumeName"].as_str()
    {
        return Some(InvolvedObject {
            kind: PV.into(),
            namespace: Namespace::all(),
            name: name.to_owned(),
        });
    }

    None
}

fn get_involved_object_from_ref(object: &Value) -> Option<InvolvedObject> {
    if let Some(kind) = object["kind"].as_str()
        && let Some(version) = object["apiVersion"].as_str()
    {
        Some(InvolvedObject {
            kind: Kind::from_api_version(kind, version),
            namespace: object["namespace"].as_str().unwrap_or_default().into(),
            name: object["name"].as_str().unwrap_or_default().to_owned(),
        })
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

    fn creation_timestamp(&self) -> Option<&Timestamp> {
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
            self.ignore_filters || self.filter_metadata.evaluate(expression)
        } else {
            self.name.contains(&context.pattern)
        }
    }
}

fn get_filter_metadata(kind: &str, group: &str, metadata: &ObjectMeta) -> SelectiveMap {
    let name = metadata
        .name
        .clone()
        .or_else(|| metadata.generate_name.clone())
        .unwrap_or_default();

    let mut result = SelectiveMap::default();

    if let Some(namespace) = metadata.namespace.clone() {
        result.insert_explicit("ns", vec![namespace]);
    } else if kind == "Namespace" && group.is_empty() {
        result.insert_explicit("ns", vec![name.clone()]);
    } else {
        result.set_optional("ns");
    }

    if let Some(labels) = metadata.labels.as_ref() {
        result.insert("l", flatten_metadata(labels));
    }

    if let Some(annotations) = metadata.annotations.as_ref() {
        result.insert("a", flatten_metadata(annotations));
    }

    result.insert("n", vec![name]);
    result
}

fn flatten_metadata(items: &BTreeMap<String, String>) -> Vec<String> {
    items
        .iter()
        .map(|(k, v)| [k.to_ascii_lowercase(), v.to_ascii_lowercase()].join("="))
        .collect::<Vec<String>>()
}
