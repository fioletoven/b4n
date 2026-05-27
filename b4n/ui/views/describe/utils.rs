use std::collections::BTreeMap;

use b4n_config::themes::{TextColors, YamlSyntaxColors};
use ratatui::style::Style;

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
