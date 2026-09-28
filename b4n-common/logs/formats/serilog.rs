use serde_json::{Map, Value};
use std::str::FromStr;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::logs::parser::LogLevel;

pub fn detect(map: &Map<String, Value>) -> bool {
    map.contains_key("@t") && (map.contains_key("@m") || map.contains_key("@mt"))
}

pub fn parse(map: Map<String, Value>) -> Option<String> {
    let timestamp = map
        .get("@t")
        .and_then(Value::as_str)
        .map(|s| {
            OffsetDateTime::parse(s, &Rfc3339)
                .map(|dt| dt.to_string())
                .unwrap_or_else(|_| s.to_string())
        })
        .unwrap_or_default();

    let level = map
        .get("@l")
        .and_then(Value::as_str)
        .and_then(|s| LogLevel::from_str(s).ok())
        .unwrap_or(LogLevel::Info);

    Some(format!("{} {}", timestamp, level))
}
