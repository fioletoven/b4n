use b4n_config::themes::{TextColors, YamlSyntaxColors};
use k8s_openapi::serde_json::Value;
use ratatui::style::Style;
use std::collections::BTreeMap;

use crate::ui::presentation::StyledLine;

/// Returns tuple with `color` and `text`.
pub fn span(color: &TextColors, text: impl Into<String>) -> (Style, String) {
    (color.into(), text.into())
}

/// Returns `none` text as a `StyledLine`.
pub fn none(colors: &YamlSyntaxColors) -> StyledLine {
    vec![span(&colors.normal, "  --none--")].into()
}

/// Creates property with `name` and `value` as a `StyledLine`.
pub fn property(colors: &YamlSyntaxColors, name: impl Into<String>, value: impl Into<String>) -> StyledLine {
    vec![
        span(&colors.property, name),
        span(&colors.normal, ": "),
        span(&colors.string, value),
    ]
    .into()
}

/// Kind used to style property value.
#[derive(Clone, Copy, PartialEq)]
pub enum ValueKind {
    String,
    Numeric,
    Boolean,
    Normal,
}

/// Creates aligned property with `name` and `value` as a `StyledLine`.
pub fn aligned_property(
    colors: &YamlSyntaxColors,
    name: &str,
    value: &str,
    kind: ValueKind,
    indent: usize,
    width: usize,
) -> StyledLine {
    let spacing = " ".repeat(width.saturating_sub(name.len()) + 1);
    let value_color = match kind {
        ValueKind::String => &colors.string,
        ValueKind::Numeric => &colors.numeric,
        ValueKind::Boolean => &colors.language,
        ValueKind::Normal => &colors.normal,
    };

    vec![
        span(&colors.normal, " ".repeat(indent)),
        span(&colors.property, name),
        span(&colors.normal, format!(":{spacing}")),
        span(value_color, value),
    ]
    .into()
}

/// Creates header with `name` as a `StyledLine`.
pub fn header(colors: &YamlSyntaxColors, name: impl Into<String>) -> StyledLine {
    vec![span(&colors.property, name), span(&colors.normal, ":")].into()
}

/// Returns a list created from the `source` map.
pub fn list(colors: &YamlSyntaxColors, source: &BTreeMap<String, String>) -> Vec<StyledLine> {
    let mut lines = Vec::with_capacity(source.len());

    for (key, value) in source {
        if key != "kubectl.kubernetes.io/last-applied-configuration" {
            lines.push(element(colors, key, value));
        }
    }

    lines
}

/// Creates list element as a `StyledLine`.
pub fn element(colors: &YamlSyntaxColors, key: impl Into<String>, value: impl Into<String>) -> StyledLine {
    vec![
        span(&colors.normal, "  - "),
        span(&colors.string, key),
        span(&colors.normal, "="),
        span(&colors.string, value),
    ]
    .into()
}

/// Converts `value` to a string.
pub fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}

/// Converts first letter of the `value` to uppercase.
pub fn uppercase_first_letter(value: &str) -> String {
    let mut c = value.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
