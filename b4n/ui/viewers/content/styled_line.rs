use b4n_common::truncate_left;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub type StyledLine = Vec<(Style, String)>;

/// Defines style handling rules when pushing a character or string slice to the end of a [`StyledLine`].
pub struct StyleFallback {
    /// If the last segment has this style, a new segment will be started instead of appending.
    pub excluded: Style,

    /// Style to apply when starting a new segment.
    pub fallback: Style,
}

pub trait StyledLineExt {
    /// Inserts a string slice into this `StyledLine` at byte position `idx`.
    fn sl_insert_str(&mut self, idx: usize, s: &str);

    /// Inserts a character into this `StyledLine` at byte position `idx`.
    fn sl_insert(&mut self, idx: usize, ch: char);

    /// Appends a given string slice to the end of this `StyledLine`.
    fn sl_push_str(&mut self, string: &str, styles: &StyleFallback);

    /// Appends a character to the back of a `StyledLine`.
    fn sl_push(&mut self, ch: char, styles: &StyleFallback);

    /// Removes a [`char`] from this `StyledLine` at byte position `idx`.
    fn sl_remove(&mut self, idx: usize);

    /// Splits [`StyledLine`] at byte position `idx` and returns the second part.
    fn get_second(&self, idx: usize) -> StyledLine;

    /// Shortens this `StyledLine` to the specified length.
    fn sl_truncate(&mut self, new_len: usize);

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
