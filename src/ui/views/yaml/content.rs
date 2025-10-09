use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
};
use tokio::sync::{mpsc::UnboundedSender, oneshot::Receiver};

use crate::{
    core::{HighlightError, HighlightRequest, HighlightResponse},
    ui::{
        ResponseEvent,
        viewers::{Content, MatchPosition, StyledLine, StyledLineExt},
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
    is_editable: bool,
}

impl YamlContent {
    /// Creates new [`YamlContent`] instance.
    pub fn new(
        styled: Vec<StyledLine>,
        plain: Vec<String>,
        highlighter: UnboundedSender<HighlightRequest>,
        is_editable: bool,
    ) -> Self {
        let max_size = plain.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let lowercase = plain.iter().map(|l| l.to_ascii_lowercase()).collect();

        Self {
            styled,
            plain,
            lowercase,
            max_size,
            highlighter,
            modified: HashSet::new(),
            requested: None,
            is_editable,
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

        self.styled[first].sl_push_str(&self.plain[second]);
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
        let split_styled = self.styled[y].get_second(x);

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
        self.styled[y].sl_truncate(x);

        self.mark_line_as_modified(y);
        self.mark_line_as_modified(insert_at);
    }

    fn remove_char_internal(&mut self, idx: usize, line_no: usize) {
        self.plain[line_no].remove(idx);
        self.lowercase[line_no].remove(idx);
        self.styled[line_no].sl_remove(idx);

        self.mark_line_as_modified(line_no);
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

    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.plain.hash(&mut hasher);
        hasher.finish()
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
        self.is_editable
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
                self.styled[y].sl_insert(r.x.index, character);
                self.mark_line_as_modified(y);
            }
        } else if y < self.plain.len() {
            if character == '\n' {
                self.add_empty_line(y + 1);
            } else {
                self.plain[y].push(character);
                self.lowercase[y].push(character.to_ascii_lowercase());
                self.styled[y].sl_push(character);
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
            self.remove_char_internal(x.index, y);
            Some((x.char, y))
        } else if y < self.plain.len() {
            let x = if is_backspace { x.saturating_sub(1) } else { x };
            if let Some(r) = get_char_position(&self.plain, x, y) {
                self.remove_char_internal(r.x.index, y);
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
