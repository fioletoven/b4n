use b4n_common::truncate_left;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

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

    /// Removes this [`StyledLine`] characters from the start to the `idx`.
    fn sl_drain_to(&mut self, idx: usize);

    /// Splits [`StyledLine`] at byte position `idx` and returns the second part.
    fn get_second(&self, idx: usize) -> StyledLine;

    /// Returns [`StyledLine`] as a [`Line`].
    fn as_line(&self, offset: usize) -> Line<'_>;
}

impl StyledLineExt for StyledLine {
    fn sl_insert_str(&mut self, idx: usize, s: &str) {
        if let Some((idx, part)) = get_part(self, idx) {
            part.insert_str(idx, s);
        }
    }

    fn sl_insert(&mut self, idx: usize, ch: char) {
        if let Some((idx, part)) = get_part(self, idx) {
            part.insert(idx, ch);
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
        for part in self {
            if current + part.1.len() > idx {
                part.1.remove(idx - current);
                return;
            }

            current += part.1.len();
        }
    }

    fn sl_truncate(&mut self, new_len: usize) {
        let mut current = 0;
        for (i, part) in self.iter_mut().enumerate() {
            if current + part.1.len() > new_len {
                part.1.truncate(new_len - current);
                if i + 1 < self.len() {
                    self.truncate(i + 1);
                }

                break;
            }

            current += part.1.len();
        }
    }

    fn sl_drain_to(&mut self, idx: usize) {
        let mut parts_to_remove = 0;
        let mut current = 0;
        for (i, part) in self.iter_mut().enumerate() {
            let len = part.1.chars().count();

            if current >= idx {
                break;
            } else if current + len >= idx {
                let left = idx.saturating_sub(current);
                if left > 0 {
                    part.1.drain(..left);
                }
            }

            parts_to_remove = i;
            current += len;
        }

        self.drain(..parts_to_remove);
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

fn get_part(line: &mut StyledLine, idx: usize) -> Option<(usize, &mut String)> {
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

        self[start_line].sl_truncate(start.x);
        self[end_line].sl_drain_to(end.x);
        remove_lines(self, start_line.saturating_add(1), end_line.saturating_sub(1));
        self.join_lines(start_line);
    }
}

fn remove_lines(lines: &mut Vec<StyledLine>, from: usize, to: usize) {
    if from <= to && from < lines.len() {
        let to = to.min(lines.len());
        lines.drain(from..=to);
    }
}
