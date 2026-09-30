use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::build_context;
use crate::logs::parser::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./logstash.tests.rs"]
mod content_tests;

const CONTEXT_FIELDS: [(&str, &str); 5] = [
    ("logger_name", ""),
    ("thread_name", "thread"),
    ("traceId", "trace"),
    ("spanId", "span"),
    ("requestId", "request"),
];

pub fn detect(map: &Map<String, Value>) -> bool {
    map.get("message").is_some_and(Value::is_string)
        && map.get("level").is_some_and(Value::is_string)
        && (map.contains_key("logger_name") || map.contains_key("thread_name") || map.contains_key("@timestamp"))
}

pub fn parse(map: &Map<String, Value>) -> Option<ParsedLogLine> {
    let level = map
        .get("level")
        .and_then(Value::as_str)
        .and_then(|level| LogLevel::from_str(level).ok())
        .unwrap_or(LogLevel::Info);

    let message = map.get("message")?.as_str()?.to_owned();

    Some(ParsedLogLine {
        level,
        message,
        context: build_context(map, &CONTEXT_FIELDS),
    })
}
