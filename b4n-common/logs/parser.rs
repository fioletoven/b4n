use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::{ecs, log4j2, logstash, serilog, zap};

/// Level of a log message.
#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "TRACE"),
            Self::Debug => write!(f, "DEBUG"),
            Self::Info => write!(f, " INFO"),
            Self::Warn => write!(f, " WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Fatal => write!(f, "FATAL"),
        }
    }
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

/// Parsed log line.
pub struct ParsedLogLine {
    pub level: LogLevel,
    pub message: String,
    pub context: Option<String>,
}

/// Log parsers that can be used to parse structured log lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogParser {
    Ecs,
    Log4j2,
    Logstash,
    Serilog,
    Zap,
}

impl LogParser {
    /// Parses a JSON map into a `ParsedLogLine` using the specific log parser.
    fn parse(self, map: &Map<String, Value>) -> Option<ParsedLogLine> {
        match self {
            Self::Ecs => ecs::parse(map),
            Self::Log4j2 => log4j2::parse(map),
            Self::Logstash => logstash::parse(map),
            Self::Serilog => serilog::parse(map),
            Self::Zap => zap::parse(map),
        }
    }
}

/// Attempts to parse a single log line using the specified or detected log parser.
pub fn parse_line(line: &str, parser: Option<LogParser>) -> Option<(ParsedLogLine, LogParser)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }

    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
        let parser = parser.or_else(|| {
            if serilog::detect(&map) {
                Some(LogParser::Serilog)
            } else if logstash::detect(&map) {
                Some(LogParser::Logstash)
            } else if log4j2::detect(&map) {
                Some(LogParser::Log4j2)
            } else if zap::detect(&map) {
                Some(LogParser::Zap)
            } else if ecs::detect(&map) {
                Some(LogParser::Ecs)
            } else {
                None
            }
        })?;

        return parser.parse(&map).map(|line| (line, parser));
    }

    None
}
