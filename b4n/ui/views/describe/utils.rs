use b4n_config::themes::{TextColors, YamlSyntaxColors};
use k8s_openapi::serde_json::{Map, Value};
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
pub fn property(colors: &YamlSyntaxColors, name: &str, value: impl Into<String>, kind: ValueKind, indent: usize) -> StyledLine {
    vec![
        span(&colors.normal, " ".repeat(indent)),
        span(&colors.property, name),
        span(&colors.normal, ": "),
        span(kind_to_color(colors, kind), value),
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
    value: impl Into<String>,
    kind: ValueKind,
    indent: usize,
    width: usize,
) -> StyledLine {
    let spacing = " ".repeat(width.saturating_sub(name.len()) + 1);
    vec![
        span(&colors.normal, " ".repeat(indent)),
        span(&colors.property, name),
        span(&colors.normal, format!(":{spacing}")),
        span(kind_to_color(colors, kind), value),
    ]
    .into()
}

/// Creates header with `name` as a `StyledLine`.
pub fn header(colors: &YamlSyntaxColors, name: impl Into<String>, indent: usize) -> StyledLine {
    vec![
        span(&colors.normal, " ".repeat(indent)),
        span(&colors.property, name),
        span(&colors.normal, ":"),
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
pub fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null => None,
        _ => Some(value.to_string()),
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

/// Joins mapped array elements in one string using `", "` separator.
pub fn map_join<F>(values: Option<&Vec<Value>>, map: F) -> Option<String>
where
    F: Fn(&Value) -> Option<String>,
{
    let filtered = values?.iter().filter_map(map).collect::<Vec<_>>();
    (!filtered.is_empty()).then_some(filtered.join(", "))
}

/// Creates string from a key value map.
pub fn map_to_string(selector: Option<&Map<String, Value>>) -> Option<String> {
    let mut items: Vec<_> = selector?
        .iter()
        .map(|(key, value)| format!("{key}={}", value_to_string(value).unwrap_or_default()))
        .collect();
    items.sort();

    (!items.is_empty()).then_some(items.join(", "))
}

/// Creates string from a selector map.
pub fn selector(selector_map: Option<&Map<String, Value>>) -> Option<String> {
    let selector = selector_map?;
    let mut items = Vec::new();

    items.extend(map_to_string(selector.get("matchLabels").and_then(Value::as_object)));
    items.extend(map_join(
        selector.get("matchExpressions").and_then(Value::as_array),
        value_to_string,
    ));
    if items.is_empty() {
        items.extend(map_to_string(selector_map));
    }

    items.sort();
    (!items.is_empty()).then_some(items.join(", "))
}

/// Returns update strategy as string.
pub fn update_strategy(strategy: Option<&Map<String, Value>>) -> Option<String> {
    let strategy = strategy?;
    let mut elements = vec![strategy.get("type").and_then(value_to_string)];

    if let Some(rolling_update) = strategy.get("rollingUpdate") {
        let max_unavailable = rolling_update.get("maxUnavailable").and_then(value_to_string);
        let max_surge = rolling_update.get("maxSurge").and_then(value_to_string);
        let partition = rolling_update.get("partition").and_then(value_to_string);

        elements.push(max_unavailable.map(|value| format!("{value} max unavailable")));
        elements.push(max_surge.map(|value| format!("{value} max surge")));
        elements.push(partition.map(|value| format!("partition {value}")));
    }

    let strategy = elements.into_iter().flatten().collect::<Vec<_>>();
    (!strategy.is_empty()).then_some(strategy.join(", "))
}

fn kind_to_color(colors: &YamlSyntaxColors, kind: ValueKind) -> &TextColors {
    match kind {
        ValueKind::String => &colors.string,
        ValueKind::Numeric => &colors.numeric,
        ValueKind::Boolean => &colors.language,
        ValueKind::Normal => &colors.normal,
    }
}
