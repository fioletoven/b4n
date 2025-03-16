use kube::api::DynamicObject;

use crate::{
    app::lists::Header,
    ui::{colors::TextColors, theme::Theme},
};

pub mod config_map;
pub mod default;
pub mod namespace;
pub mod pod;
pub mod secret;
pub mod service;

/// Returns [`ResourceData`] for provided Kubernetes resource.
pub fn get_resource_data(kind: &str, object: &DynamicObject) -> ResourceData {
    match kind {
        "ConfigMap" => config_map::data(object),
        "Namespace" => namespace::data(object),
        "Pod" => pod::data(object),
        "Secret" => secret::data(object),
        "Service" => service::data(object),
        _ => default::data(object),
    }
}

/// Returns [`Header`] for provided Kubernetes resource kind.
pub fn get_header_data(kind: &str) -> Header {
    match kind {
        "ConfigMap" => config_map::header(),
        "Namespace" => namespace::header(),
        "Pod" => pod::header(),
        "Secret" => secret::header(),
        "Service" => service::header(),
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
        let text = value.map(|v| v.into());
        let numeric = text.as_deref().map(|v| format!("{0:0>1$}", v, len));
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
