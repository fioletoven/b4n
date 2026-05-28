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

/// Creates aligned property with `name` and `value` as a `StyledLine`.
pub fn aligned_property(colors: &YamlSyntaxColors, name: &str, value: &str, indent: usize, width: usize) -> StyledLine {
    let spacing = " ".repeat(width.saturating_sub(name.len()) + 1);

    vec![
        span(&colors.normal, " ".repeat(indent)),
        span(&colors.property, name),
        span(&colors.normal, format!(":{spacing}")),
        span(&colors.string, value),
    ]
    .into()
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
