pub mod ecs;
pub mod klog;
pub mod log4j2;
pub mod logfmt;
pub mod logstash;
pub mod serilog;
pub mod zap;

use serde_json::{Map, Value};
use std::fmt::Write;

fn build_context<const N: usize>(map: &Map<String, Value>, fields: &[(&str, &str); N]) -> Option<String> {
    let fields: [Option<(&str, &Value)>; N] =
        fields.map(|(key, label)| map.get(key).filter(|value| !is_blank(value)).map(|value| (label, value)));
    build_context_from_values(fields)
}

fn build_context_from_values<const N: usize>(fields: [Option<(&str, &Value)>; N]) -> Option<String> {
    let capacity: usize = fields
        .iter()
        .flatten()
        .map(|(label, value)| 3 + label.len() + estimated_len(value))
        .sum();

    if capacity == 0 {
        return None;
    }

    let mut out = String::with_capacity(capacity);
    for (label, value) in fields.into_iter().flatten() {
        if !out.is_empty() {
            out.push_str(", ");
        }

        if !label.is_empty() {
            out.push_str(label);
            out.push('=');
        }

        push_value(value, &mut out);
    }

    Some(out)
}

fn push_value(value: &Value, out: &mut String) {
    match value {
        Value::String(s) => out.push_str(s),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Null => out.push_str("null"),
        other => {
            let _ = write!(out, "{other}");
        },
    }
}

fn estimated_len(value: &Value) -> usize {
    match value {
        Value::String(s) => s.len(),
        Value::Bool(_) => 5,
        Value::Number(_) => 8,
        Value::Null => 4,
        _ => 16,
    }
}

fn is_blank(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(values) => values.is_empty(),
        Value::Object(values) => values.is_empty(),
        _ => false,
    }
}
