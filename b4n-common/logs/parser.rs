use std::str::FromStr;

use crate::logs::formats::serilog;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" | "verbose" | "trc" => Ok(Self::Trace),
            "debug" | "dbg" => Ok(Self::Debug),
            "info" | "information" | "informational" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" | "err" => Ok(Self::Error),
            "fatal" | "critical" | "crit" | "panic" => Ok(Self::Fatal),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "TRACE"),
            Self::Debug => write!(f, "DEBUG"),
            Self::Info => write!(f, "INFO"),
            Self::Warn => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Fatal => write!(f, "FATAL"),
        }
    }
}

pub fn parse_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }

    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
        if serilog::detect(&map) {
            return serilog::parse(map);
        }
    }

    None
}
