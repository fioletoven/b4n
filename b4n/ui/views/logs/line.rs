use ansi_to_tui::IntoText;
use b4n_config::APP_NAME;
use k8s_openapi::jiff::Timestamp;
use ratatui::style::Style;
use std::fmt::{Display, Write};

use crate::ui::presentation::{ContentPosition, StyledLine};

/// Log line kind.
#[derive(PartialEq)]
pub enum LineKind {
    LogLine,
    FetchInfo,
    Error,
}

/// Represents a log message.
pub struct LogMessage {
    styled: StyledLine,
    lowercase: String,
    len: usize,
}

impl PartialEq for LogMessage {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.lowercase == other.lowercase
    }
}

/// Represents one log line.
pub struct LogLine {
    pub kind: LineKind,
    pub datetime: Timestamp,
    pub message: LogMessage,
    pub json: Option<LogMessage>,
    pub container: Option<String>,
    container_len: usize,
}

impl PartialEq for LogLine {
    fn eq(&self, other: &Self) -> bool {
        self.datetime == other.datetime
            && self.container == other.container
            && self.kind == other.kind
            && self.message == other.message
    }
}

impl LogLine {
    /// Creates new [`LogLine`] instance.
    pub fn new(datetime: Timestamp, container: Option<&str>, message: String) -> Self {
        let mut lowercase = String::with_capacity(message.len());
        let message = match message.into_text() {
            Ok(text) => text
                .lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .map(|span| (span.style, span.content.to_string()))
                .collect(),
            Err(_) => vec![(Style::default(), message)],
        };

        for (_, text) in &message {
            lowercase.push_str(&text.to_ascii_lowercase());
        }

        let message = LogMessage {
            len: lowercase.chars().count(),
            styled: message.into(),
            lowercase,
        };

        let (container, container_len) = get_container(container);

        Self {
            kind: LineKind::LogLine,
            datetime,
            message,
            json: None,
            container,
            container_len,
        }
    }

    /// Returns new error [`LogLine`] instance.
    pub fn error(datetime: Timestamp, container: Option<&str>, error: String) -> Self {
        let (container, container_len) = get_container(container);
        Self {
            kind: LineKind::Error,
            datetime,
            message: get_message(error),
            json: None,
            container,
            container_len,
        }
    }

    /// Returns new info [`LogLine`] instance.
    pub fn info(datetime: Timestamp, container: Option<&str>, info: String) -> Self {
        let (container, container_len) = get_container(container);
        Self {
            kind: LineKind::FetchInfo,
            datetime,
            message: get_message(info),
            json: None,
            container,
            container_len,
        }
    }

    /// Returns a reference to the log message segments.
    pub fn segments(&self) -> &[(Style, String)] {
        self.message.styled.segments()
    }

    /// Returns a reference to the lowercase version of the log message.
    pub fn lowercase(&self) -> &str {
        &self.message.lowercase
    }

    /// Returns whole line chars count (together with container part).
    pub fn width(&self) -> usize {
        self.message.len + self.container_width()
    }

    /// Returns container's part chars count.
    pub fn container_width(&self) -> usize {
        if self.container.is_some() { self.container_len + 2 } else { 0 }
    }

    /// Returns new [`ContentPosition`] without account container's length.
    pub fn map_position(&self, position: ContentPosition) -> ContentPosition {
        if self.container.is_some() {
            ContentPosition::new(position.x.saturating_sub(self.container_len + 2), position.y)
        } else {
            position
        }
    }

    /// Returns new bounds that have container's length.
    pub fn map_bounds(&self, bounds: Option<(usize, usize)>) -> Option<(usize, usize)> {
        if self.container.is_some() {
            bounds.map(|(x, y)| (x + self.container_len + 2, y + self.container_len + 2))
        } else {
            bounds
        }
    }

    /// Returns full line together with optional prefix.
    pub fn get_text(&self, prefix: Option<impl Display>, prefix_len: usize) -> String {
        let mut result = String::with_capacity(self.width() + if prefix.is_some() { prefix_len } else { 0 });
        if let Some(prefix) = prefix {
            write!(result, "{prefix}").unwrap();
        }

        if let Some(container) = &self.container {
            result.push_str(container);
            result.push_str(": ");
        }

        for (_, text) in self.segments() {
            result.push_str(text);
        }

        result
    }
}

fn get_container(container: Option<&str>) -> (Option<String>, usize) {
    (
        container.map(String::from),
        container.map(|c| c.chars().count()).unwrap_or_default(),
    )
}

fn get_message(text: String) -> LogMessage {
    let name = format!("[{APP_NAME}] ");
    let len = name.chars().count() + text.chars().count();
    let message = vec![(Style::default(), name), (Style::default(), text)].into();

    LogMessage {
        styled: message,
        lowercase: String::new(),
        len,
    }
}
