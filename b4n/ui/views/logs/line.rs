use ansi_to_tui::IntoText;
use k8s_openapi::jiff::Timestamp;
use ratatui::style::Style;
use std::fmt::{Display, Write};

use crate::ui::presentation::{ContentPosition, StyledLine};

/// Represents one log line.
pub struct LogLine {
    pub datetime: Timestamp,
    pub container: Option<String>,
    pub message: StyledLine,
    pub lowercase: String,
    pub is_error: bool,
    container_len: usize,
    message_len: usize,
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

        Self {
            datetime,
            container_len: container.map(|c| c.chars().count()).unwrap_or_default(),
            container: container.map(String::from),
            message_len: lowercase.chars().count(),
            message,
            lowercase,
            is_error: false,
        }
    }

    /// Returns new error [`LogLine`] instance.
    pub fn error(datetime: Timestamp, container: Option<&str>, error: String) -> Self {
        Self {
            datetime,
            container_len: container.map(|c| c.chars().count()).unwrap_or_default(),
            container: container.map(String::from),
            message_len: error.chars().count(),
            message: vec![(Style::default(), error)],
            lowercase: String::new(),
            is_error: true,
        }
    }

    /// Returns whole line chars count (together with container part).
    pub fn width(&self) -> usize {
        self.message_len + self.container_width()
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

        for (_, text) in &self.message {
            result.push_str(text);
        }

        result
    }
}

pub struct LogsChunk {
    pub end: Timestamp,
    pub lines: Vec<LogLine>,
}
