use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::build_context;
use crate::logs::parser::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./zap.tests.rs"]
mod content_tests;

const CONTEXT_FIELDS: [(&str, &str); 18] = [
    ("logger", ""),
    ("addr", "addr"),
    ("bindAddress", "bind"),
    ("caller", "caller"),
    ("controller", "controller"),
    ("controllerGroup", "group"),
    ("controllerKind", "kind"),
    ("error", "error"),
    ("kind", "kind"),
    ("name", "name"),
    ("namespace", "ns"),
    ("pod", "pod"),
    ("reconcileID", "reconcile"),
    ("request_id", "request"),
    ("source", "source"),
    ("span_id", "span"),
    ("stacktrace", "stack"),
    ("trace_id", "trace"),
];

pub fn detect(map: &Map<String, Value>) -> bool {
    map.get("msg").is_some_and(Value::is_string)
        && map.get("level").is_some_and(Value::is_string)
        && (map.contains_key("ts") || map.contains_key("time") || map.contains_key("logger") || map.contains_key("caller"))
}

pub fn parse(map: &Map<String, Value>) -> Option<ParsedLogLine> {
    let message = map.get("msg")?.as_str()?.to_owned();

    let level = map
        .get("level")
        .and_then(Value::as_str)
        .and_then(|level| LogLevel::from_str(level).ok())
        .unwrap_or(LogLevel::Info);

    Some(ParsedLogLine {
        level,
        message,
        context: build_context(map, &CONTEXT_FIELDS),
    })
}
