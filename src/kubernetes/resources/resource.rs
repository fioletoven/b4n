use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::{ResourceExt, api::DynamicObject};
use std::{collections::BTreeMap, rc::Rc};

use crate::{
    app::lists::{AGE_COLUMN_WIDTH, FilterContext, Filterable, Header, NAMESPACE, Row},
    kubernetes,
    ui::{ViewType, colors::TextColors, theme::Theme},
    utils::truncate,
};

use super::{pod, service};

#[cfg(test)]
#[path = "./resource.tests.rs"]
mod resource_tests;

/// Value for the resource extra data.
pub struct ResourceValue {
    pub text: Option<String>,
    pub number: Option<String>,
    pub is_numeric: bool,
}

impl ResourceValue {
    /// Creates new [`ResourceValue`] instance as a numeric value.
    pub fn numeric(value: Option<impl Into<String>>, len: usize) -> ResourceValue {
        let text = value.map(|v| v.into());
        let numeric = text.as_deref().map(|v| format!("{0:0>1$}", v, len));
        ResourceValue {
            text,
            number: numeric,
            is_numeric: true,
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
    pub labels: Option<BTreeMap<String, String>>,
    pub annotations: Option<BTreeMap<String, String>>,
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
            labels: None,
            annotations: None,
            data: None,
        }
    }

    /// Creates [`Resource`] from kubernetes [`DynamicObject`].
    pub fn from(kind: &str, object: DynamicObject) -> Self {
        let data = match kind {
            "Pod" => Some(pod::data(&object)),
            "Service" => Some(service::data(&object)),
            _ => None,
        };

        Self {
            age: object.creation_timestamp().as_ref().map(|t| t.0.timestamp().to_string()),
            name: object.name_any(),
            namespace: object.metadata.namespace,
            uid: object.metadata.uid,
            creation_timestamp: object.metadata.creation_timestamp,
            labels: object.metadata.labels,
            annotations: object.metadata.annotations,
            data,
        }
    }

    /// Returns [`Header`] for provided Kubernetes resource kind.
    pub fn header(kind: &str) -> Header {
        match kind {
            "Pod" => pod::header(),
            "Service" => service::header(),
            _ => Header::from(NAMESPACE.clone(), None, Rc::new([' ', 'N', 'A'])),
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
        let mut row = String::with_capacity(width + 2);
        match view {
            ViewType::Name => row.push_cell(&self.name, width, false),
            ViewType::Compact => self.get_compact_text(&mut row, header, name_width),
            ViewType::Full => self.get_full_text(&mut row, header, namespace_width, name_width),
        };

        if row.chars().count() > width {
            truncate(row.as_str(), width).to_owned()
        } else {
            row
        }
    }

    fn get_compact_text(&self, row: &mut String, header: &Header, name_width: usize) {
        row.push_cell(&self.name, name_width, false);
        row.push(' ');
        self.push_inner_text(row, header);
        row.push(' ');
        row.push_cell(
            self.creation_timestamp
                .as_ref()
                .map(kubernetes::utils::format_timestamp)
                .as_deref()
                .unwrap_or("n/a"),
            AGE_COLUMN_WIDTH + 1,
            true,
        );
    }

    fn get_full_text(&self, row: &mut String, header: &Header, namespace_width: usize, name_width: usize) {
        row.push_cell(self.namespace.as_deref().unwrap_or("n/a"), namespace_width, false);
        row.push(' ');
        self.get_compact_text(row, header, name_width);
    }

    fn push_inner_text(&self, row: &mut String, header: &Header) {
        let Some(values) = self.get_extra_values() else {
            return;
        };
        let Some(columns) = header.get_extra_columns() else {
            return;
        };
        if values.len() != columns.len() {
            return;
        }

        for i in 0..columns.len() {
            if i > 0 {
                row.push(' ');
            }

            let len = if columns[i].is_fixed {
                columns[i].data_len
            } else {
                columns[i].data_len.clamp(columns[i].min_len(), columns[i].max_len())
            };

            row.push_cell(values[i].text.as_deref().unwrap_or("n/a"), len, columns[i].to_right);
        }
    }

    fn get_extra_values(&self) -> Option<&[ResourceValue]> {
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
            values[column - 2].text.as_deref().unwrap_or("n/a")
        } else if column == values.len() + 2 {
            self.age.as_deref().unwrap_or("n/a")
        } else {
            "n/a"
        }
    }

    fn column_sort_text(&self, column: usize) -> &str {
        if let Some(values) = self.get_extra_values() {
            if column >= 2 && column <= values.len() + 1 {
                if values[column - 2].is_numeric {
                    return values[column - 2].number.as_deref().unwrap_or("n/a");
                } else {
                    return values[column - 2].text.as_deref().unwrap_or("n/a");
                }
            }
        }

        self.column_text(column)
    }
}

/// Filtering context for [`Resource`].
pub struct ResourceFilterContext {
    pattern: String,
    is_extended: bool,
}

impl FilterContext for ResourceFilterContext {
    fn restart(&mut self) {
        // Empty implementation.
    }
}

impl Filterable<ResourceFilterContext> for Resource {
    fn get_context(pattern: &str, settings: Option<&str>) -> ResourceFilterContext {
        ResourceFilterContext {
            pattern: pattern.to_owned(),
            is_extended: settings.is_some(),
        }
    }

    /// Checks if an item match a filter using the provided context.  
    /// **Note** that currently it has only a switch for normal/extended filtering.
    /// Extended filtering is when [`Some`] is provided in settings.
    fn is_matching(&self, context: &mut ResourceFilterContext) -> bool {
        if context.is_extended {
            self.name.contains(&context.pattern)
                || any(self.labels.as_ref(), &context.pattern)
                || any(self.annotations.as_ref(), &context.pattern)
        } else {
            self.name.contains(&context.pattern)
        }
    }
}

fn any(tree: Option<&BTreeMap<String, String>>, pattern: &str) -> bool {
    let Some(tree) = tree else {
        return false;
    };

    tree.keys().any(|k| k.contains(pattern)) || tree.values().any(|v| v.contains(pattern))
}

/// Extension methods for string.
pub trait StringExtensions {
    /// Appends a given cell text onto the end of this `String`.
    fn push_cell(&mut self, s: &str, len: usize, to_right: bool);
}

impl StringExtensions for String {
    fn push_cell(&mut self, s: &str, len: usize, to_right: bool) {
        if len == 0 || s.is_empty() {
            return;
        }

        let padding_len = len.saturating_sub(s.chars().count());
        if to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }

        self.push_str(truncate(s, len));

        if !to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }
    }
}
