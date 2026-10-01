use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::{build_context, is_blank, push_value};
use crate::logs::parser::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./serilog.tests.rs"]
mod content_tests;

const CONTEXT_FIELDS: [(&str, &str); 15] = [
    ("SourceContext", ""),
    ("Application", "app"),
    ("CorrelationId", "correlation"),
    ("EnvironmentName", "env"),
    ("@x", "error"),
    ("@i", "event"),
    ("MachineName", "machine"),
    ("ParentId", "parent"),
    ("ProcessId", "pid"),
    ("RequestId", "request"),
    ("RequestPath", "path"),
    ("@sp", "span"),
    ("ThreadId", "tid"),
    ("ThreadName", "thread"),
    ("@tr", "trace"),
];

pub fn detect(map: &Map<String, Value>) -> bool {
    map.contains_key("@m") || map.contains_key("@mt")
}

pub fn parse(map: &Map<String, Value>) -> Option<ParsedLogLine> {
    let level = map
        .get("@l")
        .and_then(Value::as_str)
        .and_then(|s| LogLevel::from_str(s).ok())
        .unwrap_or(LogLevel::Info);

    let msg = map.get("@m").and_then(Value::as_str).map(str::to_owned).or_else(|| {
        map.get("@mt")
            .and_then(Value::as_str)
            .map(|template| build_from_template(template, map))
    })?;

    let context_fields = CONTEXT_FIELDS.map(|(key, label)| {
        if map.get(key).is_some_and(|value| !is_blank(value)) {
            (key, label)
        } else {
            let alternate_key = match key {
                "@tr" => "TraceId",
                "@sp" => "SpanId",
                _ => return (key, label),
            };
            (alternate_key, label)
        }
    });

    Some(ParsedLogLine {
        level,
        message: msg,
        context: build_context(map, &context_fields),
    })
}

fn build_from_template(template: &str, map: &Map<String, Value>) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(pos) = remaining.find(['{', '}']) {
        rendered.push_str(&remaining[..pos]);
        remaining = &remaining[pos..];

        match remaining.as_bytes() {
            [b'{', b'{', ..] => {
                rendered.push('{');
                remaining = &remaining[2..];
            },
            [b'}', b'}', ..] => {
                rendered.push('}');
                remaining = &remaining[2..];
            },
            [b'{', ..] => {
                remaining = &remaining[1..];

                let Some(end) = remaining.find('}') else {
                    rendered.push('{');
                    rendered.push_str(remaining);
                    return rendered;
                };

                let token = &remaining[..end];
                remaining = &remaining[end + 1..];

                let property_name = token
                    .split([',', ':'])
                    .next()
                    .map(str::trim)
                    .map(|name| name.trim_start_matches(['@', '$']))
                    .unwrap_or_default();

                if property_name.is_empty() {
                    rendered.push_str("{}");
                } else if let Some(value) = map.get(property_name) {
                    push_value(value, &mut rendered);
                } else {
                    rendered.push('{');
                    rendered.push_str(token);
                    rendered.push('}');
                }
            },
            _ => {
                rendered.push('}');
                remaining = &remaining[1..];
            },
        }
    }

    rendered.push_str(remaining);
    rendered
}
