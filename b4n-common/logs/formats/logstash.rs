use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::{build_context, is_blank};
use crate::logs::parser::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./logstash.tests.rs"]
mod content_tests;

const CONTEXT_FIELDS: [(&str, &str); 14] = [
    ("logger_name", ""),
    ("caller_class_name", "class"),
    ("caller_file_name", "file"),
    ("caller_line_number", "line"),
    ("caller_method_name", "method"),
    ("correlationId", "correlation"),
    ("HOSTNAME", "host"),
    ("requestId", "request"),
    ("spanId", "span"),
    ("stack_hash", "stack_hash"),
    ("stack_trace", "stack"),
    ("tags", "tags"),
    ("thread_name", "thread"),
    ("traceId", "trace"),
];

/// Detects whether the given JSON map follows the Logstash format.
pub fn detect(map: &Map<String, Value>) -> bool {
    map.get("message").is_some_and(Value::is_string)
        && map.get("level").is_some_and(Value::is_string)
        && (map.contains_key("logger_name") || map.contains_key("thread_name") || map.contains_key("@timestamp"))
}

/// Parses a JSON map following the Logstash format into a `ParsedLogLine`.
pub fn parse(map: &Map<String, Value>) -> Option<ParsedLogLine> {
    let level = map
        .get("level")
        .and_then(Value::as_str)
        .and_then(|level| LogLevel::from_str(level).ok())
        .unwrap_or(LogLevel::Info);

    let message = map.get("message")?.as_str()?.to_owned();

    let context_fields = CONTEXT_FIELDS.map(|(key, label)| {
        if map.get(key).is_some_and(|value| !is_blank(value)) {
            (key, label)
        } else {
            let alternate_key = match key {
                "traceId" => "trace_id",
                "spanId" => "span_id",
                "requestId" => "request_id",
                "correlationId" => "correlation_id",
                "HOSTNAME" => "host",
                _ => return (key, label),
            };
            (alternate_key, label)
        }
    });

    Some(ParsedLogLine {
        level,
        message,
        context: build_context(map, &context_fields),
    })
}
