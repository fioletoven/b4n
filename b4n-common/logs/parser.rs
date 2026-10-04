use serde_json::{Map, Value};
use std::str::FromStr;

use crate::logs::formats::{ecs, klog, log4j2, logstash, serilog, zap};

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
    Klog,
    Log4j2,
    Logstash,
    Serilog,
    Zap,
}

impl LogParser {
    /// Returns `true` if this parser operates on JSON objects.
    fn is_json(self) -> bool {
        !matches!(self, Self::Klog)
    }

    /// Parses a JSON map into a `ParsedLogLine` using the specific log parser.
    fn parse_json(self, map: &Map<String, Value>) -> Option<ParsedLogLine> {
        match self {
            Self::Ecs => ecs::parse(map),
            Self::Log4j2 => log4j2::parse(map),
            Self::Logstash => logstash::parse(map),
            Self::Serilog => serilog::parse(map),
            Self::Zap => zap::parse(map),
            Self::Klog => None,
        }
    }
}

/// Detects which JSON-based parser applies to the given map, if any.
fn detect_json_parser(map: &Map<String, Value>) -> Option<LogParser> {
    if serilog::detect(map) {
        Some(LogParser::Serilog)
    } else if logstash::detect(map) {
        Some(LogParser::Logstash)
    } else if log4j2::detect(map) {
        Some(LogParser::Log4j2)
    } else if zap::detect(map) {
        Some(LogParser::Zap)
    } else if ecs::detect(map) {
        Some(LogParser::Ecs)
    } else {
        None
    }
}

/// Attempts to parse a single log line using the specified or detected log parser.
pub fn parse_line(line: &str, parser: Option<LogParser>) -> Option<(ParsedLogLine, LogParser)> {
    let trimmed = line.trim();

    if trimmed.starts_with('{') {
        parse_json_line(trimmed, parser)
    } else {
        parse_text_line(trimmed, parser)
    }
}

fn parse_json_line(trimmed: &str, parser: Option<LogParser>) -> Option<(ParsedLogLine, LogParser)> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) else {
        return None;
    };

    let parser = parser.filter(|p| p.is_json()).or_else(|| detect_json_parser(&map))?;
    parser.parse_json(&map).map(|line| (line, parser))
}

fn parse_text_line(trimmed: &str, parser: Option<LogParser>) -> Option<(ParsedLogLine, LogParser)> {
    if matches!(parser, Some(LogParser::Klog)) {
        return klog::parse(trimmed).map(|line| (line, LogParser::Klog));
    }

    if let Some(line) = klog::parse(trimmed) {
        return Some((line, LogParser::Klog));
    }

    None
}
