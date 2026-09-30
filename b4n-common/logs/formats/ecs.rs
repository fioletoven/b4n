use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::parser::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./ecs.tests.rs"]
mod content_tests;

const ECS_NAMESPACES: [&str; 5] = ["ecs", "log", "service", "process", "trace"];

const CONTEXT_FIELDS: [(&str, &str); 6] = [
    ("log.logger", ""),
    ("service.name", "service"),
    ("process.thread.name", "thread"),
    ("trace.id", "trace"),
    ("span.id", "span"),
    ("transaction.id", "transaction"),
];

pub fn detect(map: &Map<String, Value>) -> bool {
    map.get("message").is_some_and(Value::is_string) && extract_level(map).is_some() && has_ecs_marker(map)
}

pub fn parse(map: &Map<String, Value>) -> Option<ParsedLogLine> {
    let level = extract_level(map)
        .and_then(|level| LogLevel::from_str(level).ok())
        .unwrap_or(LogLevel::Info);

    let message = map.get("message")?.as_str()?.to_owned();

    Some(ParsedLogLine {
        level,
        message,
        context: build_context(map),
    })
}

fn has_ecs_marker(map: &Map<String, Value>) -> bool {
    map.keys().any(|key| {
        let namespace = key.split_once('.').map_or(key.as_str(), |(head, _)| head);
        ECS_NAMESPACES.contains(&namespace)
    })
}

fn extract_level(map: &Map<String, Value>) -> Option<&str> {
    field(map, "log.level")
        .or_else(|| map.get("level").and_then(Value::as_str))
        .or_else(|| map.get("severity").and_then(Value::as_str))
}

fn field<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    if let Some(value) = map.get(key).and_then(Value::as_str) {
        return Some(value);
    }

    let (head, rest) = key.split_once('.')?;
    let mut value = map.get(head)?;

    for part in rest.split('.') {
        value = value.as_object()?.get(part)?;
    }

    value.as_str()
}

fn build_context(map: &Map<String, Value>) -> Option<String> {
    let parts = CONTEXT_FIELDS.map(|(key, label)| field(map, key).filter(|value| !value.is_empty()).map(|value| (label, value)));
    let capacity: usize = parts
        .iter()
        .flatten()
        .map(|(label, value)| 3 + label.len() + value.len())
        .sum();

    if capacity == 0 {
        return None;
    }

    let mut out = String::with_capacity(capacity);
    for (label, value) in parts.into_iter().flatten() {
        if !out.is_empty() {
            out.push_str(", ");
        }

        if !label.is_empty() {
            out.push_str(label);
            out.push('=');
        }

        out.push_str(value);
    }

    Some(out)
}
