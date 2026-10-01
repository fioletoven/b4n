use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::build_context;
use crate::logs::parser::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./log4j2.tests.rs"]
mod content_tests;

const CONTEXT_FIELDS: [(&str, &str); 11] = [
    ("loggerName", ""),
    ("contextMap", "mdc"),
    ("contextStack", "ndc"),
    ("endOfBatch", "batch"),
    ("loggerFqcn", "fqcn"),
    ("marker", "marker"),
    ("source", "source"),
    ("thread", "thread"),
    ("threadId", "tid"),
    ("threadPriority", "priority"),
    ("thrown", "error"),
];

pub fn detect(map: &Map<String, Value>) -> bool {
    map.get("message").is_some_and(Value::is_string)
        && map.get("level").is_some_and(Value::is_string)
        && (map.contains_key("loggerName") || map.contains_key("thread") || map.contains_key("instant"))
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
