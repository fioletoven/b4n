use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::{ResourceExt, api::DynamicObject};

use crate::{
    app::lists::{Header, NAMESPACE, Row},
    kubernetes,
    ui::{ViewType, colors::TextColors, theme::Theme},
    utils::{add_padding, truncate},
};

use super::{pod, service};

#[cfg(test)]
#[path = "./resource.tests.rs"]
mod resource_tests;

/// Extra data for the kubernetes resource.
pub struct ResourceData {
    pub is_job: bool,
    pub is_completed: bool,
    pub is_ready: bool,
    pub is_terminating: bool,
    pub extra_values: Box<[Option<String>]>,
}

impl ResourceData {
    /// Returns [`TextColors`] for the current resource state.
    fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        if self.is_completed {
            theme.colors.line.completed.get_specific(is_active, is_selected)
        } else if self.is_ready {
            theme.colors.line.ready.get_specific(is_active, is_selected)
        } else if self.is_terminating {
            theme.colors.line.terminating.get_specific(is_active, is_selected)
        } else {
            theme.colors.line.in_progress.get_specific(is_active, is_selected)
        }
    }
}

/// Represents kubernetes resource of any kind.
pub struct Resource {
    pub uid: Option<String>,
    pub name: String,
    pub namespace: Option<String>,
    pub age: Option<String>,
    pub creation_timestamp: Option<Time>,
    pub data: Option<ResourceData>,
}

impl Resource {
    /// Creates light [`Resource`] version just with name.
    pub fn new(name: &str) -> Self {
        Self {
            uid: Some(format!("_{}_", name)),
            name: name.to_owned(),
            namespace: None,
            age: None,
            creation_timestamp: None,
            data: None,
        }
    }

    /// Creates [`Resource`] from kubernetes [`DynamicObject`].
    pub fn from(kind: &str, object: &DynamicObject) -> Self {
        let data = match kind {
            "Pod" => Some(pod::data(object)),
            "Service" => Some(service::data(object)),
            _ => None,
        };
        Self {
            uid: object.metadata.uid.clone(),
            name: object.name_any(),
            namespace: object.namespace(),
            creation_timestamp: object.creation_timestamp(),
            age: object.creation_timestamp().as_ref().map(kubernetes::utils::format_timestamp),
            data,
        }
    }

    /// Returns [`Header`] for provided Kubernetes resource kind.
    pub fn header(kind: &str) -> Header {
        match kind {
            "Pod" => pod::header(),
            "Service" => service::header(),
            _ => Header::from(NAMESPACE.clone(), None),
        }
    }

    /// Returns [`TextColors`] for this kubernetes resource considering `theme` and other data.
    pub fn get_colors(&self, theme: &Theme, is_active: bool, is_selected: bool) -> TextColors {
        if let Some(data) = &self.data {
            data.get_colors(theme, is_active, is_selected)
        } else {
            theme.colors.line.ready.get_specific(is_active, is_selected)
        }
    }

    /// Builds and returns the whole row of values for this kubernetes resource.
    pub fn get_text(&self, view: ViewType, header: &Header, width: usize, namespace_width: usize, name_width: usize) -> String {
        let text = match view {
            ViewType::Name => self.get_name(width),
            ViewType::Compact => self.get_compact_text(&self.get_inner_text(header), name_width),
            ViewType::Full => self.get_full_text(&self.get_inner_text(header), namespace_width, name_width),
        };

        if text.chars().count() > width {
            truncate(text.as_str(), width).to_owned()
        } else {
            text
        }
    }

    fn get_compact_text(&self, inner_text: &str, name_width: usize) -> String {
        format!(
            "{1:<0$} {2} {3:>7}",
            name_width,
            truncate(self.name.as_str(), name_width),
            inner_text,
            self.creation_timestamp
                .as_ref()
                .map(kubernetes::utils::format_timestamp)
                .as_deref()
                .unwrap_or("n/a")
        )
    }

    fn get_full_text(&self, inner_text: &str, namespace_width: usize, name_width: usize) -> String {
        format!(
            "{1:<0$} {3:<2$} {4} {5:>7}",
            namespace_width,
            truncate(self.namespace.as_deref().unwrap_or("n/a"), namespace_width),
            name_width,
            truncate(self.name.as_str(), name_width),
            inner_text,
            self.creation_timestamp
                .as_ref()
                .map(kubernetes::utils::format_timestamp)
                .as_deref()
                .unwrap_or("n/a")
        )
    }

    fn get_inner_text(&self, header: &Header) -> String {
        let Some(values) = self.get_extra_values() else {
            return String::new();
        };
        let Some(columns) = header.get_extra_columns() else {
            return String::new();
        };
        if values.len() != columns.len() {
            return String::new();
        }

        let text_len = columns.iter().map(|c| c.max_len + 1).sum::<usize>();
        let mut text = String::with_capacity(text_len * 2);
        for i in 0..columns.len() {
            if i > 0 {
                text.push(' ');
            }

            let mut len = columns[i].data_len;
            if !columns[i].is_fixed {
                len = columns[i].data_len.clamp(columns[i].min_len, columns[i].max_len);
            }

            text.push_str(&add_padding(values[i].as_deref().unwrap_or("n/a"), len, columns[i].to_right));
        }

        text
    }

    fn get_extra_values(&self) -> Option<&[Option<String>]> {
        self.data.as_ref().map(|data| &*data.extra_values)
    }
}

impl Row for Resource {
    fn uid(&self) -> Option<&str> {
        self.uid.as_deref()
    }

    fn group(&self) -> &str {
        self.namespace.as_deref().unwrap_or_default()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> &str {
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
            values[column - 2].as_deref().unwrap_or("n/a")
        } else if column == values.len() + 2 {
            self.age.as_deref().unwrap_or("n/a")
        } else {
            "n/a"
        }
    }
}
