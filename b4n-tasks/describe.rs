use b4n_config::themes::YamlSyntaxColors;
use ratatui_core::style::Style;
use time::format_description::well_known::{Rfc2822, Rfc3339};

/// Parses `kubectl describe` output into styled spans.
pub fn highlight_describe(lines: &[String], colors: &YamlSyntaxColors) -> Vec<Vec<(Style, String)>> {
    let mut styled_lines = Vec::with_capacity(lines.len());
    let mut plain_mode = false;
    let mut last_key_indent: Option<usize> = None;

    for line in lines {
        if plain_mode || line.trim().is_empty() {
            styled_lines.push(vec![((&colors.string).into(), line.clone())]);
            continue;
        }

        match parse_describe_line(line, colors, last_key_indent) {
            Some((spans, indent)) => {
                styled_lines.push(spans);
                last_key_indent = Some(indent);
                if line.trim_start().starts_with("Events:") {
                    plain_mode = true;
                }
            },
            None => {
                styled_lines.push(vec![((&colors.string).into(), line.clone())]);
            },
        }
    }

    styled_lines
}

/// Attempts to parse a single `key: value` or `key:` line.\
/// Returns `None` if the line doesn't match the expected pattern or indentation rules.\
/// Returns `Some((spans, indentation))` on success.
fn parse_describe_line(
    line: &str,
    colors: &YamlSyntaxColors,
    last_indent: Option<usize>,
) -> Option<(Vec<(Style, String)>, usize)> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();

    if last_indent.is_some_and(|last_indent| indent > last_indent + 2) {
        return None;
    }

    let (key, after_colon) = if let Some(pos) = trimmed.find(": ") {
        (&trimmed[..pos], &trimmed[pos + 1..])
    } else {
        let stripped = trimmed.strip_suffix(':')?;
        (stripped, "")
    };

    if key.is_empty() {
        return None;
    }

    let mut spans = Vec::new();

    if indent > 0 {
        spans.push(((&colors.normal).into(), " ".repeat(indent)));
    }

    spans.push(((&colors.property).into(), key.to_string()));
    spans.push(((&colors.normal).into(), ":".to_string()));

    let value = after_colon.trim();
    if value.is_empty() {
        return Some((spans, indent));
    }

    let value_style = classify_value(value, colors);
    spans.push((value_style, after_colon.to_string()));

    Some((spans, indent))
}

fn classify_value(value: &str, colors: &YamlSyntaxColors) -> Style {
    if matches!(value, "true" | "false" | "True" | "False")
        || matches!(value, "null" | "~" | "<none>" | "<unset>" | "<unknown>" | "<nil>")
    {
        return (&colors.language).into();
    }

    if is_number(value) || is_k8s_quantity(value) {
        return (&colors.numeric).into();
    }

    if is_timestamp(value) {
        return (&colors.timestamp).into();
    }

    (&colors.string).into()
}

fn is_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s).strip_suffix('%').unwrap_or(s);
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn is_k8s_quantity(s: &str) -> bool {
    const SUFFIXES: &[&str] = &[
        "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "k", "M", "G", "T", "P", "E", "m", "n", "s", "b",
    ];

    match SUFFIXES.iter().find_map(|&sfx| s.strip_suffix(sfx)) {
        Some(n) => is_number(n),
        None => false,
    }
}

fn is_timestamp(s: &str) -> bool {
    time::OffsetDateTime::parse(s, &Rfc3339).is_ok() || time::OffsetDateTime::parse(s, &Rfc2822).is_ok()
}
