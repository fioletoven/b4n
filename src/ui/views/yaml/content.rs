use ratatui::style::Style;
use std::collections::HashSet;
use tokio::sync::{mpsc::UnboundedSender, oneshot::Receiver};

use crate::{
    core::{HighlightError, HighlightRequest, HighlightResponse},
    ui::{
        ResponseEvent,
        viewers::{Content, MatchPosition, StyledLine},
    },
};

/// Number of lines before and after the modified section to include in the re-highlighting process.
const HIGHLIGHT_CONTEXT_LINES_NO: usize = 200;

/// Styled YAML content.
pub struct YamlContent {
    pub styled: Vec<StyledLine>,
    pub plain: Vec<String>,
    pub lowercase: Vec<String>,
    max_size: usize,
    highlighter: UnboundedSender<HighlightRequest>,
    modified: HashSet<usize>,
    requested: Option<RequestedHighlight>,
}

impl YamlContent {
    /// Creates new [`YamlContent`] instance.
    pub fn new(
        styled: Vec<StyledLine>,
        plain: Vec<String>,
        lowercase: Vec<String>,
        highlighter: UnboundedSender<HighlightRequest>,
        max_size: usize,
    ) -> Self {
        Self {
            styled,
            plain,
            lowercase,
            max_size,
            highlighter,
            modified: HashSet::new(),
            requested: None,
        }
    }

    fn mark_line_as_modified(&mut self, line_no: usize) {
        if let Some(line) = self.plain.get(line_no) {
            self.modified.insert(line_no);
            self.max_size = self.max_size.max(line.len());
        }
    }

    fn add_empty_line(&mut self, line_no: usize) {
        if line_no < self.plain.len() {
            self.plain.insert(line_no, String::new());
            self.lowercase.insert(line_no, String::new());
            self.styled.insert(line_no, Vec::new());
        } else {
            self.plain.push(String::new());
            self.lowercase.push(String::new());
            self.styled.push(Vec::new());
        }

        self.mark_line_as_modified(line_no);
    }

    fn join_lines(&mut self, first: usize, second: usize) -> (usize, usize) {
        let new_x = self.plain[first].chars().count();

        styled_append(&mut self.styled[first], &self.plain[second]);
        self.styled.remove(second);

        let text = self.plain.remove(second);
        self.plain[first].push_str(&text);

        let text = self.lowercase.remove(second);
        self.lowercase[first].push_str(&text);

        self.mark_line_as_modified(first);
        self.mark_line_as_modified(second);

        (new_x, first)
    }

    fn split_lines(&mut self, x: usize, y: usize) {
        let split_plain = self.plain[y][x..].to_string();
        let split_lowercase = self.lowercase[y][x..].to_string();
        let split_styled = styled_split(&self.styled[y], x);

        let insert_at = y + 1;
        if insert_at < self.plain.len() {
            self.plain.insert(insert_at, split_plain);
            self.lowercase.insert(insert_at, split_lowercase);
            self.styled.insert(insert_at, split_styled);
        } else {
            self.plain.push(split_plain);
            self.lowercase.push(split_lowercase);
            self.styled.push(split_styled);
        }

        self.plain[y].truncate(x);
        self.lowercase[y].truncate(x);
        styled_truncate(&mut self.styled[y], x);

        self.mark_line_as_modified(y);
        self.mark_line_as_modified(insert_at);
    }
}

impl Content for YamlContent {
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine] {
        if start >= self.styled.len() {
            &[]
        } else if start + count >= self.styled.len() {
            &self.styled[start..]
        } else {
            &self.styled[start..start + count]
        }
    }

    fn len(&self) -> usize {
        self.styled.len()
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
        self.max_size + 1
    }

    fn line_size(&self, line_no: usize) -> usize {
        self.plain.get(line_no).map(|l| l.chars().count()).unwrap_or_default()
    }

    fn is_editable(&self) -> bool {
        true
    }

    fn insert_char(&mut self, x: usize, y: usize, character: char) {
        if let Some(r) = get_char_position(&self.plain, x, y) {
            if character == '\n' {
                if r.x.index == 0 {
                    self.add_empty_line(y);
                } else {
                    self.split_lines(r.x.index, y);
                }
            } else {
                self.plain[y].insert(r.x.index, character);
                self.lowercase[y].insert(r.x.index, character.to_ascii_lowercase());
                styled_insert(&mut self.styled[y], r.x.index, character);
                self.mark_line_as_modified(y);
            }
        } else if y < self.plain.len() {
            if character == '\n' {
                self.add_empty_line(y + 1);
            } else {
                self.plain[y].push(character);
                self.lowercase[y].push(character.to_ascii_lowercase());
                styled_push(&mut self.styled[y], character);
                self.mark_line_as_modified(y);
            }
        }
    }

    fn remove_char(&mut self, x: usize, y: usize, is_backspace: bool) -> Option<(usize, usize)> {
        if is_backspace && x == 0 {
            if y > 0 && y < self.plain.len() {
                return Some(self.join_lines(y - 1, y));
            } else {
                return Some((x, y));
            }
        }

        if let Some(r) = get_char_position(&self.plain, x, y) {
            let x = if is_backspace { r.x_prev } else { r.x };

            self.plain[y].remove(x.index);
            self.lowercase[y].remove(x.index);
            styled_remove(&mut self.styled[y], x.index);
            self.mark_line_as_modified(y);

            Some((x.char, y))
        } else if y < self.plain.len() {
            let x = if is_backspace { x.saturating_sub(1) } else { x };
            if let Some(r) = get_char_position(&self.plain, x, y) {
                self.plain[y].remove(r.x.index);
                self.lowercase[y].remove(r.x.index);
                styled_remove(&mut self.styled[y], r.x.index);

                self.mark_line_as_modified(y);

                Some((r.x.char, y))
            } else if y + 1 < self.plain.len() {
                Some(self.join_lines(y, y + 1))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn process_tick(&mut self) -> ResponseEvent {
        if let Some(requested) = &mut self.requested
            && let Ok(response) = requested.response.try_recv()
        {
            if self.modified.is_empty()
                && let Ok(response) = response
            {
                self.styled.splice(requested.start..=requested.end, response.styled);
            }

            self.requested = None;
        }

        if self.requested.is_none() && !self.modified.is_empty() {
            let first = self.modified.iter().min().copied().unwrap_or_default();
            let last = self.modified.iter().max().copied().unwrap_or_default();
            let start = first.saturating_sub(HIGHLIGHT_CONTEXT_LINES_NO);
            let end = last
                .saturating_add(HIGHLIGHT_CONTEXT_LINES_NO)
                .min(self.plain.len().saturating_sub(1));

            let (tx, rx) = tokio::sync::oneshot::channel();

            let _ = self.highlighter.send(HighlightRequest::Partial {
                start: first.saturating_sub(start),
                lines: self.plain[start..=end].to_vec(),
                response: tx,
            });

            self.modified.clear();
            self.requested = Some(RequestedHighlight {
                start: first,
                end,
                response: rx,
            });
        }

        ResponseEvent::Handled
    }
}

struct RequestedHighlight {
    pub start: usize,
    pub end: usize,
    pub response: Receiver<Result<HighlightResponse, HighlightError>>,
}

/// Inserts a character into this `StyledLine` at byte position `idx`.
fn styled_insert(line: &mut StyledLine, idx: usize, ch: char) {
    let mut current = 0;
    for part in line {
        if current + part.1.len() >= idx {
            part.1.insert(idx - current, ch);
            return;
        }

        current += part.1.len();
    }
}

/// Appends a given string slice to the end of this `StyledLine`.
fn styled_append(line: &mut StyledLine, string: &str) {
    if let Some(part) = line.last_mut() {
        part.1.push_str(string);
    } else {
        line.push((Style::default(), string.to_owned()));
    }
}

/// Appends a character to the back of a `StyledLine`.
fn styled_push(line: &mut StyledLine, ch: char) {
    if let Some(part) = line.last_mut() {
        part.1.push(ch);
    } else {
        line.push((Style::default(), ch.to_string()));
    }
}

/// Removes a [`char`] from this `StyledLine` at byte position `idx`.
fn styled_remove(line: &mut StyledLine, idx: usize) {
    let mut current = 0;
    for part in line {
        if current + part.1.len() > idx {
            part.1.remove(idx - current);
            return;
        }

        current += part.1.len();
    }
}

/// Splits [`StyledLine`] at byte position `idx` and returns the second part.
fn styled_split(line: &StyledLine, idx: usize) -> StyledLine {
    let mut result = Vec::new();
    let mut current = 0;
    let mut is_found = false;
    for part in line {
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

/// Shortens this `StyledLine` to the specified length.
fn styled_truncate(line: &mut StyledLine, new_len: usize) {
    let mut current = 0;
    for (i, part) in line.iter_mut().enumerate() {
        if current + part.1.len() > new_len {
            part.1.truncate(new_len - current);
            if i + 1 < line.len() {
                line.truncate(i + 1);
            }

            break;
        }

        current += part.1.len();
    }
}

#[derive(Default)]
struct CharPosition {
    pub char: usize,
    pub index: usize,
}

#[derive(Default)]
struct PositionSet {
    pub x_prev: CharPosition,
    pub x: CharPosition,
}

fn get_char_position(lines: &[String], idx: usize, line_no: usize) -> Option<PositionSet> {
    let line = lines.get(line_no)?;
    let mut result_set = PositionSet::default();

    for (char_idx, (byte_idx, _)) in line.char_indices().enumerate() {
        if char_idx + 1 == idx {
            result_set.x_prev = CharPosition {
                char: char_idx,
                index: byte_idx,
            };
        }

        if char_idx == idx {
            result_set.x = CharPosition {
                char: char_idx,
                index: byte_idx,
            };
            return Some(result_set);
        }
    }

    None
}
