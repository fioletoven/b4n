use b4n_common::truncate_left;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::ops::{Bound, RangeBounds};

use crate::ui::presentation::Selection;

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
    /// Returns the number of characters in the [`StyledLine`].
    fn sl_len(&self) -> usize;

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
    fn sl_drain(&mut self, range: impl RangeBounds<usize>);

    /// Splits [`StyledLine`] at byte position `idx` and returns the second part.
    fn get_second(&self, idx: usize) -> StyledLine;

    /// Returns [`StyledLine`] as a [`Line`].
    fn as_line(&self, offset: usize) -> Line<'_>;
}

impl StyledLineExt for StyledLine {
    fn sl_len(&self) -> usize {
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

    fn sl_drain(&mut self, range: impl RangeBounds<usize>) {
        let start = start_from_bound(&range);
        let end = end_from_bound(&range);

        let mut remove_start = self.len();
        let mut remove_end = 0;
        let mut current = 0;

        for (i, (_, span)) in self.iter_mut().enumerate() {
            let span_len = span.chars().count();

            if current + span_len <= start {
                // pass
            } else if current <= start {
                let drain_from = start.saturating_sub(current);
                if current + span_len >= end {
                    let drain_to = end.saturating_sub(current);
                    span.drain(drain_from..drain_to);
                    remove_start = i + 1;
                } else if drain_from == 0 {
                    remove_start = i
                } else {
                    span.drain(drain_from..);
                    remove_start = i + 1;
                }
            } else if current >= end {
                break;
            } else if current + span_len >= end {
                let drain_to = end.saturating_sub(current);
                if drain_to > 0 {
                    span.drain(..drain_to);
                }

                break;
            }

            remove_end = i;
            current += span_len;
        }

        if matches!(range.end_bound(), Bound::Unbounded) {
            remove_end = self.len().saturating_sub(1);
        }

        if remove_start <= remove_end {
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
    fn remove_text(&mut self, range: Selection);
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

    fn remove_text(&mut self, range: Selection) {
        let (start, end) = range.sorted();
        let start_line = start.y.min(self.len().saturating_sub(1));
        let end_line = end.y.min(self.len().saturating_sub(1));

        if start_line == end_line {
            if self[end_line].sl_len() == end.x {
                self[end_line].sl_drain(start.x..);
                self.join_lines(end_line);
            } else {
                self[end_line].sl_drain(start.x..=end.x);
            }
        } else {
            self[start_line].sl_truncate(start.x);
            if self[end_line].sl_len() == end.x {
                self.remove(end_line);
            } else {
                self[end_line].sl_drain(..=end.x);
            }

            remove_lines(self, start_line.saturating_add(1), end_line.saturating_sub(1));
            self.join_lines(start_line);
        }
    }
}

fn remove_lines(lines: &mut Vec<StyledLine>, from: usize, to: usize) {
    if from <= to && from < lines.len() {
        let to = to.min(lines.len());
        lines.drain(from..=to);
    }
}

fn start_from_bound<R: RangeBounds<usize>>(range: &R) -> usize {
    match range.start_bound() {
        Bound::Included(i) => *i,
        Bound::Excluded(i) => i + 1,
        Bound::Unbounded => 0,
    }
}

fn end_from_bound<R: RangeBounds<usize>>(range: &R) -> usize {
    match range.end_bound() {
        Bound::Included(i) => i + 1,
        Bound::Excluded(i) => *i,
        Bound::Unbounded => usize::MAX,
    }
}
