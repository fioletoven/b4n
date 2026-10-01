use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::{build_context_from_values, is_blank};
use crate::logs::parser::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./ecs.tests.rs"]
mod content_tests;

const ECS_NAMESPACES: [&str; 5] = ["ecs", "log", "service", "process", "trace"];

const CONTEXT_FIELDS: [(&str, &str); 19] = [
    ("log.logger", ""),
    ("error.message", "error"),
    ("error.stack_trace", "stack"),
    ("error.type", "error_type"),
    ("event.dataset", "dataset"),
    ("host.name", "host"),
    ("log.origin.file.line", "line"),
    ("log.origin.file.name", "file"),
    ("log.origin.function", "function"),
    ("process.pid", "pid"),
    ("process.thread.id", "tid"),
    ("process.thread.name", "thread"),
    ("service.environment", "env"),
    ("service.name", "service"),
    ("service.node.name", "node"),
    ("service.version", "version"),
    ("span.id", "span"),
    ("trace.id", "trace"),
    ("transaction.id", "transaction"),
];

/// Detects whether the given JSON map follows the ECS (Elastic Common Schema) format.
pub fn detect(map: &Map<String, Value>) -> bool {
    map.get("message").is_some_and(Value::is_string) && extract_level(map).is_some() && has_ecs_marker(map)
}

/// Parses a JSON map following the ECS format into a `ParsedLogLine`.
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
        .and_then(Value::as_str)
        .or_else(|| map.get("level").and_then(Value::as_str))
        .or_else(|| map.get("severity").and_then(Value::as_str))
}

fn field<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    if let Some(value) = map.get(key).filter(|value| !is_blank(value)) {
        return Some(value);
    }

    let (head, rest) = key.split_once('.')?;
    let mut value = map.get(head)?;

    for part in rest.split('.') {
        value = value.as_object()?.get(part)?;
    }

    Some(value)
}

fn build_context(map: &Map<String, Value>) -> Option<String> {
    let fields = CONTEXT_FIELDS.map(|(key, label)| field(map, key).filter(|value| !is_blank(value)).map(|value| (label, value)));
    build_context_from_values(fields)
}
