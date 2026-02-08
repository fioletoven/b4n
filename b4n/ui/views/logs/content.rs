use b4n_common::{slice_from, slice_to, substring};
use b4n_config::themes::LogsSyntaxColors;
use ratatui::style::Style;

use crate::ui::presentation::{Content, ContentPosition, MatchPosition, Selection, StyledLine};
use crate::ui::views::logs::{LogLine, LogsChunk};

pub const INITIAL_LOGS_VEC_SIZE: usize = 5_000;
pub const TIMESTAMP_TEXT_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f ";
pub const TIMESTAMP_TEXT_LENGTH: usize = 24;

/// Logs content for [`LogsView`].
pub struct LogsContent {
    show_timestamps: bool,
    colors: LogsSyntaxColors,
    lines: Vec<LogLine>,
    lowercase: Vec<String>,
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
            lines: Vec::with_capacity(INITIAL_LOGS_VEC_SIZE),
            lowercase: Vec::with_capacity(INITIAL_LOGS_VEC_SIZE),
            page: Vec::default(),
            max_size: 0,
            start: 0,
            count: 0,
        }
    }

    pub fn set_timestamps(&mut self, enabled: bool) {
        if self.show_timestamps != enabled {
            self.show_timestamps = enabled;
            self.count = 0;
        }
    }

    pub fn toggle_timestamps(&mut self) {
        self.show_timestamps = !self.show_timestamps;
        self.count = 0;

        if self.show_timestamps {
            self.max_size = self.max_size.saturating_add(TIMESTAMP_TEXT_LENGTH);
        } else {
            self.max_size = self.max_size.saturating_sub(TIMESTAMP_TEXT_LENGTH);
        }
    }

    pub fn show_timestamps(&self) -> bool {
        self.show_timestamps
    }

    pub fn add_logs_chunk(&mut self, chunk: LogsChunk) {
        self.count = 0; // force re-render current logs page

        for line in chunk.lines {
            let width = if self.show_timestamps {
                line.message.chars().count() + TIMESTAMP_TEXT_LENGTH
            } else {
                line.message.chars().count()
            };

            if self.max_size < width {
                self.max_size = width;
            }

            self.lowercase.push(line.message.to_ascii_lowercase());
            self.lines.push(line);
        }
    }

    fn style_log_line(&self, line: &LogLine) -> Vec<(Style, String)> {
        let log_colors = if line.is_error {
            &self.colors.error
        } else {
            &self.colors.string
        };

        if self.show_timestamps {
            vec![
                (
                    (&self.colors.timestamp).into(),
                    line.datetime.strftime(TIMESTAMP_TEXT_FORMAT).to_string(),
                ),
                (log_colors.into(), line.message.clone()),
            ]
        } else {
            vec![(log_colors.into(), line.message.clone())]
        }
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
        let (start, end) = range.map_or_else(|| (0, self.lines.len()), |(s, e)| (s.y, e.y));
        let start_line = start.min(self.lines.len().saturating_sub(1));
        let end_line = end.min(self.lines.len().saturating_sub(1));
        let (start, end) = range.map_or_else(|| (0, self.line_size(end_line).saturating_sub(1)), |(s, e)| (s.x, e.x));

        let mut result = String::new();
        for i in start_line..=end_line {
            let line = &self.lines[i];
            if i == start_line || i == end_line {
                let text = if self.show_timestamps {
                    format!("{}{}", line.datetime.strftime(TIMESTAMP_TEXT_FORMAT), line.message)
                } else {
                    line.message.clone()
                };

                if i == start_line && i == end_line {
                    result.push_str(substring(&text, start, (end + 1).saturating_sub(start)));
                    if text.chars().count() < end + 1 {
                        result.push('\n');
                    }
                } else if i == start_line {
                    result.push_str(slice_from(&text, start));
                    result.push('\n');
                } else if i == end_line {
                    result.push_str(slice_to(&text, end + 1));
                    if text.chars().count() < end + 1 {
                        result.push('\n');
                    }
                }
            } else {
                if self.show_timestamps {
                    result.push_str(&line.datetime.strftime(TIMESTAMP_TEXT_FORMAT).to_string());
                }

                result.push_str(&line.message);
                result.push('\n');
            }
        }

        result
    }

    fn search_first(&self, pattern: &str) -> Option<MatchPosition> {
        let pattern = pattern.to_ascii_lowercase();
        for (y, line) in self.lowercase.iter().enumerate() {
            if let Some(x) = line.find(&pattern) {
                return Some(MatchPosition::new(x, y, pattern.len()));
            }
        }

        None
    }

    fn search(&self, pattern: &str) -> Vec<MatchPosition> {
        let pattern = pattern.to_ascii_lowercase();
        let mut matches = Vec::new();
        for (y, line) in self.lowercase.iter().enumerate() {
            for (x, _) in line.match_indices(&pattern) {
                matches.push(MatchPosition::new(x, y, pattern.len()));
            }
        }

        matches
    }

    fn max_size(&self) -> usize {
        self.max_size
    }

    fn line_size(&self, line_no: usize) -> usize {
        let size = self.lines.get(line_no).map(|l| l.message.chars().count()).unwrap_or_default();
        if self.show_timestamps {
            size + TIMESTAMP_TEXT_LENGTH
        } else {
            size
        }
    }

    fn word_bounds(&self, position: ContentPosition) -> Option<(usize, usize)> {
        if let Some(line) = self.lines.get(position.y) {
            if self.show_timestamps {
                let idx = position.x.saturating_sub(TIMESTAMP_TEXT_LENGTH);
                let bounds = b4n_common::word_bounds(&line.message, idx);
                bounds.map(|(x, y)| (x + TIMESTAMP_TEXT_LENGTH, y + TIMESTAMP_TEXT_LENGTH))
            } else {
                b4n_common::word_bounds(&line.message, position.x)
            }
        } else {
            None
        }
    }
}
