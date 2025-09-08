use k8s_openapi::{
    apimachinery::pkg::apis::meta::v1::Time,
    chrono::{DateTime, Utc},
    serde_json::{Value, from_value},
};
use kube::api::DynamicObject;
use std::borrow::Cow;

use crate::{
    kubernetes::{
        metrics::{CpuMetrics, MemoryMetrics},
        resources::CrdColumns,
        utils::format_datetime,
        watchers::Statistics,
    },
    ui::{colors::TextColors, lists::Header, theme::Theme},
};

pub mod config_map;
pub mod container;
pub mod crd;
pub mod custom_resource;
pub mod daemon_set;
pub mod default;
pub mod deployment;
pub mod event;
pub mod job;
pub mod namespace;
pub mod node;
pub mod node_metrics;
pub mod pod;
pub mod pod_metrics;
pub mod replica_set;
pub mod secret;
pub mod service;
pub mod stateful_set;

/// Returns [`ResourceData`] for provided Kubernetes resource.
pub fn get_resource_data(
    kind: &str,
    group: &str,
    crd: Option<&CrdColumns>,
    stats: &Statistics,
    object: &DynamicObject,
) -> ResourceData {
    if let Some(crd) = crd {
        return custom_resource::data(crd, object);
    }

    match kind {
        "ConfigMap" => config_map::data(object),
        "CustomResourceDefinition" => crd::data(object),
        "DaemonSet" => daemon_set::data(object),
        "Deployment" => deployment::data(object),
        "Event" => event::data(object),
        "Job" => job::data(object),
        "Namespace" => namespace::data(object),
        "Node" => node::data(object, stats),
        "NodeMetrics" if group == "metrics.k8s.io" => node_metrics::data(object),
        "Pod" => pod::data(object, stats),
        "PodMetrics" if group == "metrics.k8s.io" => pod_metrics::data(object),
        "ReplicaSet" => replica_set::data(object),
        "Secret" => secret::data(object),
        "Service" => service::data(object),
        "StatefulSet" => stateful_set::data(object),

        _ => default::data(object),
    }
}

/// Returns [`Header`] for provided Kubernetes resource kind.
pub fn get_header_data(kind: &str, group: &str, crd: Option<&CrdColumns>, has_metrics: bool) -> Header {
    if let Some(crd) = crd {
        return custom_resource::header(crd);
    }

    match kind {
        "ConfigMap" => config_map::header(),
        "CustomResourceDefinition" => crd::header(),
        "DaemonSet" => daemon_set::header(),
        "Deployment" => deployment::header(),
        "Event" => event::header(),
        "Job" => job::header(),
        "Namespace" => namespace::header(),
        "Node" => node::header(has_metrics),
        "NodeMetrics" if group == "metrics.k8s.io" => node_metrics::header(),
        "Pod" => pod::header(has_metrics),
        "PodMetrics" if group == "metrics.k8s.io" => pod_metrics::header(),
        "ReplicaSet" => replica_set::header(),
        "Secret" => secret::header(),
        "Service" => service::header(),
        "StatefulSet" => stateful_set::header(),

        "Container" => container::header(has_metrics),
        _ => default::header(),
    }
}

/// Value for the resource extra data.
#[derive(Default)]
pub struct ResourceValue {
    text: Option<String>,
    sort_text: Option<String>,
    time: Option<Time>,
    is_time: bool,
}

impl ResourceValue {
    /// Creates new [`ResourceValue`] instance as a number value.
    pub fn number(value: Option<f64>, len: u32) -> Self {
        let value = value.unwrap_or_default();
        let sort_value = value + (10u64.pow(len) as f64);
        Self {
            text: Some(format!("{:0.precision$}", value, precision = 3)),
            sort_text: Some(format!(
                "{:0>width$.precision$}",
                sort_value,
                width = (len as usize) + 5,
                precision = 3
            )),
            ..Default::default()
        }
    }

    /// Creates new [`ResourceValue`] instance as an integer value.
    pub fn integer(value: Option<i64>, len: u32) -> Self {
        let value = value.unwrap_or_default();
        let sort_value = value + 10i64.pow(len);
        let sort = format!("{:0>width$}", sort_value, width = (len as usize) + 1);
        Self {
            text: Some(value.to_string()),
            sort_text: Some(sort),
            ..Default::default()
        }
    }

    /// Creates new [`ResourceValue`] instance as a time value.
    pub fn time(value: Value) -> Self {
        let time = from_value::<Time>(value).ok();
        let sort = time.as_ref().map(|t| t.0.timestamp().to_string());
        Self {
            time,
            sort_text: sort,
            is_time: true,
            ..Default::default()
        }
    }

    /// Returns resource value that can be used for sorting.
    pub fn sort_text(&self) -> &str {
        if let Some(sort_text) = &self.sort_text {
            sort_text
        } else {
            self.text.as_deref().unwrap_or("n/a")
        }
    }

    /// Returns resource text.
    pub fn text(&self) -> Cow<'_, str> {
        if self.is_time {
            Cow::Owned(
                self.time
                    .as_ref()
                    .map_or("n/a".to_owned(), crate::kubernetes::utils::format_timestamp),
            )
        } else {
            Cow::Borrowed(self.text.as_deref().unwrap_or("n/a"))
        }
    }
}

impl From<Option<String>> for ResourceValue {
    fn from(value: Option<String>) -> Self {
        ResourceValue {
            text: value,
            ..Default::default()
        }
    }
}

impl From<String> for ResourceValue {
    fn from(value: String) -> Self {
        ResourceValue {
            text: Some(value),
            ..Default::default()
        }
    }
}

impl From<Option<&str>> for ResourceValue {
    fn from(value: Option<&str>) -> Self {
        ResourceValue {
            text: value.map(String::from),
            ..Default::default()
        }
    }
}

impl From<&str> for ResourceValue {
    fn from(value: &str) -> Self {
        ResourceValue {
            text: Some(value.into()),
            ..Default::default()
        }
    }
}

impl From<bool> for ResourceValue {
    fn from(value: bool) -> Self {
        ResourceValue {
            text: Some(value.to_string()),
            ..Default::default()
        }
    }
}

impl From<Option<&DateTime<Utc>>> for ResourceValue {
    fn from(value: Option<&DateTime<Utc>>) -> Self {
        Self {
            text: value.map(format_datetime),
            sort_text: value.map(|v| v.timestamp_millis().to_string()),
            ..Default::default()
        }
    }
}

impl From<Option<CpuMetrics>> for ResourceValue {
    fn from(value: Option<CpuMetrics>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl From<CpuMetrics> for ResourceValue {
    fn from(value: CpuMetrics) -> Self {
        let text = value.millicores();
        let sort = format!("{:0>width$}", text, width = 10);
        Self {
            text: Some(text),
            sort_text: Some(sort),
            ..Default::default()
        }
    }
}

impl From<Option<MemoryMetrics>> for ResourceValue {
    fn from(value: Option<MemoryMetrics>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl From<MemoryMetrics> for ResourceValue {
    fn from(value: MemoryMetrics) -> Self {
        Self {
            text: Some(value.rounded()),
            sort_text: Some(format!("{:0>width$}", value.value, width = 25)),
            ..Default::default()
        }
    }
}

/// Extra data for the kubernetes resource.
#[derive(Default)]
pub struct ResourceData {
    pub is_completed: bool,
    pub is_ready: bool,
    pub is_terminating: bool,
    pub extra_values: Box<[ResourceValue]>,
    pub one_container: Option<String>,
}

impl ResourceData {
    /// Creates new [`ResourceData`] instance.
    pub fn new(values: Box<[ResourceValue]>, is_terminating: bool) -> Self {
        Self {
            extra_values: values,
            is_ready: !is_terminating,
            is_terminating,
            ..Default::default()
        }
    }

    /// Returns [`TextColors`] for the current resource state.
    pub fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        if self.is_completed {
            theme.colors.line.completed.get_specific(is_active, is_selected)
        } else if self.is_terminating {
            theme.colors.line.terminating.get_specific(is_active, is_selected)
        } else if self.is_ready {
            theme.colors.line.ready.get_specific(is_active, is_selected)
        } else {
            theme.colors.line.in_progress.get_specific(is_active, is_selected)
        }
    }
}
