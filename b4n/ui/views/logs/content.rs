use b4n_common::{slice_from, slice_to, substring};
use b4n_config::themes::LogsSyntaxColors;
use k8s_openapi::jiff::Timestamp;
use ratatui::style::Style;
use std::collections::HashMap;
use std::fmt::Write;

use crate::ui::presentation::{Content, ContentPosition, MatchPosition, Selection, StyledLine};
use crate::ui::views::logs::line::{LineKind, LogLine};

pub const INITIAL_LOGS_VEC_SIZE: usize = 5_000;
pub const TIMESTAMP_TEXT_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f ";
pub const TIMESTAMP_TEXT_LENGTH: usize = 24;

#[cfg(test)]
#[path = "./content.tests.rs"]
mod content_tests;

/// Logs content for [`LogsView`].
pub struct LogsContent {
    show_timestamps: bool,
    colors: LogsSyntaxColors,
    container_colors: HashMap<String, usize>,
    lines: Vec<LogLine>,
    page: Vec<StyledLine>,
    max_size: usize,
    start: usize,
    count: usize,
}

impl LogsContent {
    /// Returns new [`LogsContent`] instance.
    pub fn new(colors: LogsSyntaxColors) -> Self {
        Self {
            show_timestamps: true,
            colors,
            container_colors: HashMap::new(),
            lines: Vec::with_capacity(INITIAL_LOGS_VEC_SIZE),
            page: Vec::default(),
            max_size: 0,
            start: 0,
            count: 0,
        }
    }

    /// Sets if showing timestamps is enabled.
    pub fn set_timestamps(&mut self, enabled: bool) {
        if self.show_timestamps != enabled {
            self.show_timestamps = enabled;
            self.count = 0;
        }
    }

    /// Toggles showing timestamps.
    pub fn toggle_timestamps(&mut self) {
        self.show_timestamps = !self.show_timestamps;
        self.count = 0;

        if self.show_timestamps {
            self.max_size = self.max_size.saturating_add(TIMESTAMP_TEXT_LENGTH);
        } else {
            self.max_size = self.max_size.saturating_sub(TIMESTAMP_TEXT_LENGTH);
        }
    }

    /// Returns `true` if showing timestamps is enabled.
    pub fn show_timestamps(&self) -> bool {
        self.show_timestamps
    }

    /// Returns first line as a tuple of time and line lowercase text.
    pub fn get_first_line(&self) -> Option<&LogLine> {
        self.lines.iter().find(|&line| line.kind == LineKind::LogLine)
    }

    /// Returns timestamp of the first log line in this content.
    pub fn get_first_timestamp(&self) -> Option<Timestamp> {
        self.lines.first().map(|l| l.datetime)
    }

    /// Returns timestamp of the last log line in this content.
    pub fn get_last_timestamp(&self) -> Option<Timestamp> {
        self.lines.last().map(|l| l.datetime)
    }

    /// Add a single log line, maintaining sorted order and deduplicating.
    /// Returns position where this line was added.
    pub fn add_log_line(&mut self, line: LogLine) -> Option<usize> {
        self.update_max_size(&line);

        if self.lines.is_empty() {
            self.lines.push(line);
            return Some(0);
        }

        if sort_key(&line) >= sort_key(self.lines.last().unwrap()) {
            let tail_start = {
                let reversed = self.lines.iter().rev();
                self.lines.len() - reversed.take_while(|l| sort_key(l) >= sort_key(&line)).count()
            };

            let is_duplicate = self.lines[tail_start..].iter().any(|existing| existing == &line);
            if !is_duplicate {
                self.lines.push(line);
            }

            return Some(self.lines.len().saturating_sub(1));
        }

        self.merge_sorted(line)
    }

    /// Insert a single new line into the sorted `self.lines`, deduplicating.
    fn merge_sorted(&mut self, incoming: LogLine) -> Option<usize> {
        let pos = self.lines.partition_point(|l| sort_key(l) <= sort_key(&incoming));

        let start = self.lines[..pos]
            .iter()
            .rposition(|l| sort_key(l) != sort_key(&incoming))
            .map_or(0, |i| i + 1);

        let end = self.lines[pos..]
            .iter()
            .position(|l| sort_key(l) != sort_key(&incoming))
            .map_or(self.lines.len(), |i| pos + i);

        let is_duplicate = self.lines[start..end].iter().any(|existing| existing == &incoming);
        if !is_duplicate {
            self.lines.insert(pos, incoming);
            return Some(pos);
        }

        None
    }

    fn update_max_size(&mut self, line: &LogLine) {
        let timestamp_extra = if self.show_timestamps { TIMESTAMP_TEXT_LENGTH } else { 0 };
        let size = line.width() + timestamp_extra;

        self.count = 0; // force re-render current logs page
        self.max_size = self.max_size.max(size);
    }

    fn style_log_line(&self, line: &LogLine) -> Vec<(Style, String)> {
        let log_colors = match line.kind {
            LineKind::LogLine => &self.colors.string,
            LineKind::FetchInfo => &self.colors.info,
            LineKind::Error => &self.colors.error,
        };

        let mut result = Vec::new();
        if self.show_timestamps {
            result.push((
                (&self.colors.timestamp).into(),
                line.datetime.strftime(TIMESTAMP_TEXT_FORMAT).to_string(),
            ));
        }

        if let Some(container) = line.container.as_deref() {
            let idx = self.container_colors.get(container).copied().unwrap_or(0) % self.colors.containers.len().max(1);
            let container_colors = self.colors.containers.get(idx).unwrap_or(log_colors);
            result.push((container_colors.into(), container.to_owned()));
            result.push(((&self.colors.string).into(), ": ".to_owned()));
        }

        let style: Style = log_colors.into();
        result.extend(line.message.iter().map(|(s, t)| (style.patch(*s), t.clone())));

        result
    }
}

impl Content for LogsContent {
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine] {
        if start >= self.lines.len() {
            return &[];
        }

        let end = start + count;
        let end = if end >= self.lines.len() { self.lines.len() } else { end };
        if self.start != start || self.count != count {
            self.start = start;
            self.count = count;
            self.page = Vec::with_capacity(end - start);

            for line in &self.lines[start..end] {
                ensure_container_has_color(&mut self.container_colors, line.container.as_deref());
                self.page.push(self.style_log_line(line));
            }
        }

        &self.page
    }

    fn len(&self) -> usize {
        self.lines.len()
    }

    fn hash(&self) -> u64 {
        0
    }

    fn to_plain_text(&self, range: Option<Selection>) -> String {
        let range = range.map(|r| r.sorted());
        let (start_y, end_y) = range.map_or_else(|| (0, self.lines.len()), |(s, e)| (s.y, e.y));
        let start_line = start_y.min(self.lines.len().saturating_sub(1));
        let end_line = end_y.min(self.lines.len().saturating_sub(1));
        let (start_x, end_x) = range.map_or_else(|| (0, self.line_size(end_line).saturating_sub(1)), |(s, e)| (s.x, e.x));

        let mut result = String::new();
        for i in start_line..=end_line {
            let line = &self.lines[i];
            if i == start_line || i == end_line {
                let dt = self.show_timestamps.then(|| line.datetime.strftime(TIMESTAMP_TEXT_FORMAT));
                let text = line.get_text(dt, TIMESTAMP_TEXT_LENGTH);

                if i == start_line && i == end_line {
                    result.push_str(substring(&text, start_x, (end_x + 1).saturating_sub(start_x)));
                    if text.chars().count() < end_x + 1 {
                        result.push('\n');
                    }
                } else if i == start_line {
                    result.push_str(slice_from(&text, start_x));
                    result.push('\n');
                } else if i == end_line {
                    result.push_str(slice_to(&text, end_x + 1));
                    if text.chars().count() < end_x + 1 {
                        result.push('\n');
                    }
                }
            } else {
                if self.show_timestamps {
                    write!(result, "{}", line.datetime.strftime(TIMESTAMP_TEXT_FORMAT)).unwrap();
                }

                if let Some(container) = &line.container {
                    result.push_str(container);
                    result.push_str(": ");
                }

                for (_, text) in &line.message {
                    result.push_str(text);
                }

                result.push('\n');
            }
        }

        result
    }

    fn search_first(&self, pattern: &str) -> Option<MatchPosition> {
        let pattern = pattern.to_ascii_lowercase();
        for (y, line) in self.lines.iter().enumerate() {
            if let Some(x) = line.lowercase.find(&pattern) {
                return Some(MatchPosition::new(x + line.container_width(), y, pattern.len()));
            }
        }

        None
    }

    fn search(&self, pattern: &str) -> Vec<MatchPosition> {
        let pattern = pattern.to_ascii_lowercase();
        let mut matches = Vec::new();
        for (y, line) in self.lines.iter().enumerate() {
            for (x, _) in line.lowercase.match_indices(&pattern) {
                matches.push(MatchPosition::new(x + line.container_width(), y, pattern.len()));
            }
        }

        matches
    }

    fn max_size(&self) -> usize {
        self.max_size
    }

    fn line_size(&self, line_no: usize) -> usize {
        let size = self.lines.get(line_no).map(LogLine::width).unwrap_or_default();
        if self.show_timestamps {
            size + TIMESTAMP_TEXT_LENGTH
        } else {
            size
        }
    }

    fn word_bounds(&self, position: ContentPosition) -> Option<(usize, usize)> {
        if let Some(line) = self.lines.get(position.y) {
            let position = line.map_position(position);
            if self.show_timestamps {
                let idx = position.x.saturating_sub(TIMESTAMP_TEXT_LENGTH);
                let bounds = line.map_bounds(b4n_common::word_bounds(&line.lowercase, idx));
                bounds.map(|(x, y)| (x + TIMESTAMP_TEXT_LENGTH, y + TIMESTAMP_TEXT_LENGTH))
            } else {
                line.map_bounds(b4n_common::word_bounds(&line.lowercase, position.x))
            }
        } else {
            None
        }
    }
}

/// Get deterministic ordering for lines with identical timestamps.
#[inline]
fn sort_key(line: &LogLine) -> impl Ord + '_ {
    (line.datetime, &line.container)
}

fn ensure_container_has_color(container_colors: &mut HashMap<String, usize>, container: Option<&str>) {
    if let Some(container) = container
        && !container_colors.contains_key(container)
    {
        let idx = container_colors.len();
        container_colors.insert(container.to_owned(), idx);
    }
}
