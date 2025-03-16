use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::{ResourceExt, api::DynamicObject};
use std::collections::BTreeMap;

use crate::{
    app::lists::{AGE_COLUMN_WIDTH, FilterContext, Filterable, Header, Row},
    kubernetes,
    ui::{ViewType, colors::TextColors, theme::Theme},
    utils::{
        logical_expressions::{Expression, ExpressionExtensions, parse},
        truncate,
    },
};

use super::{ResourceData, ResourceValue, get_header_data, get_resource_data};

#[cfg(test)]
#[path = "./resource.tests.rs"]
mod resource_tests;

/// Represents kubernetes resource of any kind.
#[derive(Default)]
pub struct Resource {
    pub uid: Option<String>,
    pub name: String,
    pub namespace: Option<String>,
    pub age: Option<String>,
    pub creation_timestamp: Option<Time>,
    pub filter_metadata: Vec<String>,
    pub data: Option<ResourceData>,
}

impl Resource {
    /// Creates light [`Resource`] version just with name.
    pub fn new(name: &str) -> Self {
        Self {
            uid: Some(format!("_{}_", name)),
            name: name.to_owned(),
            ..Default::default()
        }
    }

    /// Creates [`Resource`] from kubernetes [`DynamicObject`].
    pub fn from(kind: &str, object: DynamicObject) -> Self {
        let data = Some(get_resource_data(kind, &object));
        let filter = get_filter_metadata(&object);

        Self {
            age: object.creation_timestamp().as_ref().map(|t| t.0.timestamp().to_string()),
            name: object.name_any(),
            namespace: object.metadata.namespace,
            uid: object.metadata.uid,
            creation_timestamp: object.metadata.creation_timestamp,
            filter_metadata: filter,
            data,
        }
    }

    /// Returns [`Header`] for provided Kubernetes resource kind.
    pub fn header(kind: &str) -> Header {
        get_header_data(kind)
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
                .unwrap_or(" "),
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
        let Some(columns) = header.get_extra_columns() else {
            return;
        };
        if let Some(values) = self.get_extra_values() {
            if values.len() != columns.len() {
                return;
            }

            for i in 0..columns.len() {
                if i > 0 {
                    row.push(' ');
                }

                row.push_cell(
                    values[i].text.as_deref().unwrap_or("n/a"),
                    columns[i].len(),
                    columns[i].to_right,
                );
            }
        } else {
            for (i, column) in columns.iter().enumerate() {
                if i > 0 {
                    row.push(' ');
                }

                (0..column.len()).for_each(|_| row.push(' '));
            }
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
                return values[column - 2].value();
            }
        }

        self.column_text(column)
    }
}

/// Filtering context for [`Resource`].
pub struct ResourceFilterContext {
    pattern: String,
    extended: Option<Expression>,
}

impl FilterContext for ResourceFilterContext {
    fn restart(&mut self) {
        // Empty implementation.
    }
}

impl Filterable<ResourceFilterContext> for Resource {
    fn get_context(pattern: &str, settings: Option<&str>) -> ResourceFilterContext {
        let expression = if let Some(settings) = settings {
            if settings.contains('e') {
                match parse(pattern) {
                    Ok(expression) => Some(expression),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        ResourceFilterContext {
            pattern: pattern.to_owned(),
            extended: expression,
        }
    }

    /// Checks if an item match a filter using the provided context.  
    /// Extended filtering is when `e` is provided in settings.  
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

#[inline]
fn flatten_metadata(items: &BTreeMap<String, String>) -> Vec<String> {
    items
        .iter()
        .map(|(k, v)| [k.to_ascii_lowercase(), v.to_ascii_lowercase()].join(": "))
        .collect::<Vec<String>>()
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
