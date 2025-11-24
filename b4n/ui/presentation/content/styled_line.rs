use b4n_common::truncate_left;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::ui::presentation::utils::char_to_index;
use crate::ui::presentation::{ContentPosition, Selection};

pub type StyledLine = Vec<(Style, String)>;

#[cfg(test)]
#[path = "./styled_line.tests.rs"]
mod styled_line_tests;

/// Defines style handling rules when pushing a character or string slice to the end of a [`StyledLine`].
pub struct StyleFallback {
    /// If the last segment has this style, a new segment will be started instead of appending.
    pub excluded: Style,

    /// Style to apply when starting a new segment.
    pub fallback: Style,
}

/// Extension methods for `StyledLine`.
pub trait StyledLineExt {
    /// Returns byte index from char index for the [`StyledLine`].
    fn char_to_index(&self, char_idx: usize) -> Option<usize>;

    /// Returns length of the [`StyledLine`].
    fn sl_len(&self) -> usize;

    /// Returns the number of characters in the [`StyledLine`].
    fn sl_chars_len(&self) -> usize;

    /// Inserts a string slice into this [`StyledLine`] at byte position `idx`.
    fn sl_insert_str(&mut self, idx: usize, s: &str);

    /// Inserts a character into this [`StyledLine`] at byte position `idx`.
    fn sl_insert(&mut self, idx: usize, ch: char);

    /// Appends a given string slice to the end of this [`StyledLine`].
    fn sl_push_str(&mut self, string: &str, styles: &StyleFallback);

    /// Appends a character to the back of a [`StyledLine`].
    fn sl_push(&mut self, ch: char, styles: &StyleFallback);

    /// Removes a [`char`] from this [`StyledLine`] at byte position `idx`.
    fn sl_remove(&mut self, idx: usize);

    /// Shortens this [`StyledLine`] to the specified length.
    fn sl_truncate(&mut self, new_len: usize);

    /// Removes the specified range from the [`StyledLine`] in bulk.
    fn sl_drain(&mut self, start: Option<usize>, end: Option<usize>);

    /// Splits [`StyledLine`] at byte position `idx` and returns the second part.
    fn get_second(&self, idx: usize) -> StyledLine;

    /// Returns [`StyledLine`] as a [`Line`].
    fn as_line(&self, offset: usize) -> Line<'_>;
}

impl StyledLineExt for StyledLine {
    fn char_to_index(&self, char_idx_22: usize) -> Option<usize> {
        let mut left = char_idx_22 + 1;
        let mut idx = 0;

        for (_, span) in self {
            let mut tmp_char = 0;
            let mut tmp_idx = 0;

            for (char_idx, (byte_idx, _)) in span.char_indices().enumerate() {
                tmp_char = char_idx + 1;
                tmp_idx = byte_idx + 1;
                if char_idx + 1 == left {
                    return Some(idx + tmp_idx - 1);
                }
            }

            left -= tmp_char;
            idx += tmp_idx;
        }

        None
    }

    fn sl_len(&self) -> usize {
        self.iter().map(|s| s.1.len()).sum()
    }

    fn sl_chars_len(&self) -> usize {
        self.iter().map(|s| s.1.chars().count()).sum()
    }

    fn sl_insert_str(&mut self, idx: usize, s: &str) {
        if let Some((idx, span)) = get_span(self, idx) {
            span.insert_str(idx, s);
        }
    }

    fn sl_insert(&mut self, idx: usize, ch: char) {
        if let Some((idx, span)) = get_span(self, idx) {
            span.insert(idx, ch);
        }
    }

    fn sl_push_str(&mut self, string: &str, styles: &StyleFallback) {
        if let Some(part) = self.last_mut()
            && part.0 != styles.excluded
        {
            part.1.push_str(string);
        } else {
            self.push((styles.fallback, string.to_owned()));
        }
    }

    fn sl_push(&mut self, ch: char, styles: &StyleFallback) {
        if let Some(part) = self.last_mut()
            && part.0 != styles.excluded
        {
            part.1.push(ch);
        } else {
            self.push((styles.fallback, ch.to_string()));
        }
    }

    fn sl_remove(&mut self, idx: usize) {
        let mut current = 0;
        for (_, span) in self {
            if current + span.len() > idx {
                span.remove(idx - current);
                return;
            }

            current += span.len();
        }
    }

    fn sl_truncate(&mut self, new_len: usize) {
        let mut current = 0;
        for (i, (_, span)) in self.iter_mut().enumerate() {
            if current + span.len() > new_len {
                span.truncate(new_len - current);
                if i + 1 < self.len() {
                    self.truncate(i + 1);
                }

                break;
            }

            current += span.len();
        }
    }

    fn sl_drain(&mut self, range_start: Option<usize>, range_end: Option<usize>) {
        let start = range_start.unwrap_or_default();
        let end = range_end.unwrap_or(usize::MAX);

        let mut remove_start = self.len();
        let mut remove_end = 0;
        let mut current = 0;

        for (i, (_, span)) in self.iter_mut().enumerate() {
            let span_len = span.chars().count();

            if current + span_len <= start {
                // pass
            } else if current <= start {
                let drain_from = char_to_index(span, start.saturating_sub(current)).unwrap_or(0);
                if current + span_len > end {
                    let drain_to = char_to_index(span, end.saturating_sub(current)).unwrap_or(0);
                    span.drain(drain_from..drain_to);
                    remove_start = i + 1;
                } else if drain_from == 0 && current + span_len <= end {
                    remove_start = i;
                } else {
                    span.drain(drain_from..);
                    remove_start = i + 1;
                }
            } else if current >= end {
                break;
            } else if current + span_len == end {
                remove_end = i;
                break;
            } else if current + span_len > end {
                let drain_to = char_to_index(span, end.saturating_sub(current)).unwrap_or(0);
                if drain_to > 0 {
                    span.drain(..drain_to);
                }

                break;
            }

            remove_end = i;
            current += span_len;
        }

        if range_end.is_none() {
            remove_end = self.len().saturating_sub(1);
        }

        if remove_start <= remove_end && remove_end < self.len() {
            self.drain(remove_start..=remove_end);
        }
    }

    fn get_second(&self, idx: usize) -> StyledLine {
        let mut result = Vec::new();
        let mut current = 0;
        let mut is_found = false;
        for part in self {
            if is_found {
                result.push((part.0, part.1.clone()));
            } else if current + part.1.len() > idx {
                result.push((part.0, part.1[idx - current..].to_string()));
                is_found = true;
            }

            current += part.1.len();
        }

        result
    }

    fn as_line(&self, offset: usize) -> Line<'_> {
        let mut spans = Vec::new();

        let mut current = 0;
        for part in self {
            let len = part.1.chars().count();

            if current >= offset {
                spans.push(Span::styled(&part.1, part.0));
            } else if current + len >= offset {
                let left = offset.saturating_sub(current);
                let new_len = len.saturating_sub(left);
                if new_len > 0 {
                    spans.push(Span::styled(truncate_left(&part.1, new_len), part.0));
                }
            }

            current += len;
        }

        Line::from(spans)
    }
}

fn get_span(line: &mut StyledLine, idx: usize) -> Option<(usize, &mut String)> {
    let mut current = 0;
    for part in line {
        if current + part.1.len() >= idx {
            return Some((idx - current, &mut part.1));
        }

        current += part.1.len();
    }

    None
}

/// Extension methods for `Vec<StyledLine>`.
pub trait VecStyledLineExt {
    /// Converts the given value to a `String`.
    fn to_string(&self) -> String;

    /// Appends the content of the next line to the line at `line_no` and removes the next line.
    fn join_lines(&mut self, line_no: usize);

    /// Removes the specified `range` from the vector of `StyledLine`s.
    fn remove_text(&mut self, range: &Selection);

    /// Inserts specified `text` at `position` to the vector of `StyledLine`s.
    fn insert_text(&mut self, position: ContentPosition, text: &[String], styles: &StyleFallback);
}

impl VecStyledLineExt for Vec<StyledLine> {
    fn to_string(&self) -> String {
        self.iter()
            .map(|line| line.iter().map(|span| span.1.as_str()).collect::<Vec<_>>().join(""))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn join_lines(&mut self, line_no: usize) {
        if line_no + 1 < self.len() {
            let (left, right) = self.split_at_mut(line_no + 1);
            left[line_no].append(&mut right[0]);
            self.remove(line_no + 1);
        }
    }

    fn remove_text(&mut self, range: &Selection) {
        let (start, end) = range.sorted();
        let start_line = start.y.min(self.len().saturating_sub(1));
        let end_line = end.y.min(self.len().saturating_sub(1));
        let is_eol = self[end_line].sl_chars_len() <= end.x;

        if start_line == end_line {
            self[end_line].sl_drain(Some(start.x), Some(end.x + 1));
            if is_eol {
                self.join_lines(end_line);
            }
        } else {
            let mut remove_start = false;

            if let Some(start) = self[start_line].char_to_index(start.x) {
                self[start_line].sl_truncate(start);
                remove_start = start == 0;
            }

            self[end_line].sl_drain(None, Some(end.x + 1));

            if is_eol {
                self.join_lines(end_line);
            }

            remove_lines(self, start_line.saturating_add(1), end_line.saturating_sub(1));

            if remove_start {
                self.remove(start_line);
            } else {
                self.join_lines(start_line);
            }
        }
    }

    fn insert_text(&mut self, position: ContentPosition, text: &[String], styles: &StyleFallback) {
        match text.len() {
            0 => (),
            1 => insert_line(self, position, &text[0], styles),
            _ => insert_lines(self, position, text, styles),
        }
    }
}

fn remove_lines(lines: &mut Vec<StyledLine>, from: usize, to: usize) {
    if from <= to && from < lines.len() {
        let to = to.min(lines.len());
        lines.drain(from..=to);
    }
}

fn insert_line(lines: &mut Vec<StyledLine>, position: ContentPosition, text: &str, styles: &StyleFallback) {
    if lines.is_empty() || position.y >= lines.len() {
        lines.push(vec![(styles.fallback, text.to_owned())]);
        return;
    }

    if lines.len() == 1 && lines[0].is_empty() {
        lines[0] = vec![(styles.fallback, text.to_owned())];
        return;
    }

    if let Some(line) = lines.get_mut(position.y) {
        if let Some(x) = line.char_to_index(position.x) {
            line.sl_insert_str(x, text);
        } else {
            line.sl_push_str(text, styles);
        }
    }
}

fn insert_lines(lines: &mut Vec<StyledLine>, position: ContentPosition, text: &[String], styles: &StyleFallback) {
    if lines.is_empty() || (lines.len() == 1 && lines[0].is_empty()) {
        *lines = add_style(text, styles.fallback);
        return;
    }

    if position.y >= lines.len() {
        lines.append(&mut add_style(text, styles.fallback));
        return;
    }

    let first_line = &text[0];
    let last_line = &text[text.len().saturating_sub(1)];
    let last_line = if let Some(x) = lines[position.y].char_to_index(position.x) {
        let mut rest = lines[position.y].get_second(x);
        lines[position.y].sl_truncate(x);
        lines[position.y].sl_push_str(first_line, styles);
        rest.sl_insert_str(0, last_line);
        rest
    } else {
        vec![(styles.fallback, last_line.to_owned())]
    };

    let mut middle_lines = if text.len() > 2 {
        let mut lines = add_style(&text[1..text.len().saturating_sub(1)], styles.fallback);
        lines.push(last_line);
        lines
    } else {
        vec![last_line]
    };

    let insert_at = position.y + 1;
    if insert_at < lines.len() {
        lines.splice(insert_at..insert_at, middle_lines);
    } else {
        lines.append(&mut middle_lines);
    }
}

fn add_style(text: &[String], style: Style) -> Vec<StyledLine> {
    text.iter().map(|line| vec![(style, line.to_owned())]).collect::<Vec<_>>()
}
