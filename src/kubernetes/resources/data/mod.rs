use k8s_openapi::chrono::{DateTime, Utc};
use kube::api::DynamicObject;

use crate::{
    kubernetes::utils::format_datetime,
    ui::{colors::TextColors, lists::Header, theme::Theme},
};

pub mod config_map;
pub mod container;
pub mod daemon_set;
pub mod default;
pub mod deployment;
pub mod event;
pub mod job;
pub mod namespace;
pub mod pod;
pub mod replica_set;
pub mod secret;
pub mod service;
pub mod stateful_set;

/// Returns [`ResourceData`] for provided Kubernetes resource.
pub fn get_resource_data(kind: &str, object: &DynamicObject) -> ResourceData {
    match kind {
        "ConfigMap" => config_map::data(object),
        "DaemonSet" => daemon_set::data(object),
        "Deployment" => deployment::data(object),
        "Event" => event::data(object),
        "Job" => job::data(object),
        "Namespace" => namespace::data(object),
        "Pod" => pod::data(object),
        "ReplicaSet" => replica_set::data(object),
        "Secret" => secret::data(object),
        "Service" => service::data(object),
        "StatefulSet" => stateful_set::data(object),

        _ => default::data(object),
    }
}

/// Returns [`Header`] for provided Kubernetes resource kind.
pub fn get_header_data(kind: &str) -> Header {
    match kind {
        "ConfigMap" => config_map::header(),
        "DaemonSet" => daemon_set::header(),
        "Deployment" => deployment::header(),
        "Event" => event::header(),
        "Job" => job::header(),
        "Namespace" => namespace::header(),
        "Pod" => pod::header(),
        "ReplicaSet" => replica_set::header(),
        "Secret" => secret::header(),
        "Service" => service::header(),
        "StatefulSet" => stateful_set::header(),

        "Container" => container::header(),
        _ => default::header(),
    }
}

/// Value for the resource extra data.
pub struct ResourceValue {
    pub text: Option<String>,
    pub number: Option<String>,
    pub is_numeric: bool,
}

impl ResourceValue {
    /// Creates new [`ResourceValue`] instance as a numeric value.
    pub fn numeric(value: Option<impl Into<String>>, len: usize) -> Self {
        let text = value.map(Into::into);
        let numeric = text.as_deref().map(|v| format!("{v:0>len$}"));
        Self {
            text,
            number: numeric,
            is_numeric: true,
        }
    }

    /// Creates new [`ResourceValue`] instance as a datetime value.
    pub fn datetime(value: Option<&DateTime<Utc>>) -> Self {
        let text = value.map(format_datetime);
        let numeric = value.map(|v| v.timestamp_millis().to_string());
        Self {
            text,
            number: numeric,
            is_numeric: true,
        }
    }

    /// Returns resource value.
    pub fn value(&self) -> &str {
        if self.is_numeric {
            self.number.as_deref().unwrap_or("NaN")
        } else {
            self.text.as_deref().unwrap_or("n/a")
        }
    }
}

impl From<Option<String>> for ResourceValue {
    fn from(value: Option<String>) -> Self {
        ResourceValue {
            text: value,
            number: None,
            is_numeric: false,
        }
    }
}

impl From<String> for ResourceValue {
    fn from(value: String) -> Self {
        ResourceValue {
            text: Some(value),
            number: None,
            is_numeric: false,
        }
    }
}

impl From<&str> for ResourceValue {
    fn from(value: &str) -> Self {
        ResourceValue {
            text: Some(value.into()),
            number: None,
            is_numeric: false,
        }
    }
}

impl From<bool> for ResourceValue {
    fn from(value: bool) -> Self {
        ResourceValue {
            text: Some(value.to_string()),
            number: None,
            is_numeric: false,
        }
    }
}

/// Extra data for the kubernetes resource.
pub struct ResourceData {
    pub is_job: bool,
    pub is_completed: bool,
    pub is_ready: bool,
    pub is_terminating: bool,
    pub extra_values: Box<[ResourceValue]>,
}

impl ResourceData {
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
