use ratatui::style::Style;
use std::collections::HashSet;
use tokio::sync::{mpsc::UnboundedSender, oneshot::Receiver};

use crate::{
    core::{HighlightError, HighlightRequest, HighlightResponse},
    ui::{
        ResponseEvent,
        views::{
            content::{Content, StyledLine},
            content_search::MatchPosition,
        },
    },
};

/// Number of lines before and after the modified section to include in the re-highlighting process.
const HIGHLIGHT_CONTEXT_LINES_NO: usize = 200;

/// Styled YAML content.
pub struct YamlContent {
    pub styled: Vec<StyledLine>,
    pub plain: Vec<String>,
    pub lowercase: Vec<String>,
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
    ) -> Self {
        Self {
            styled,
            plain,
            lowercase,
            highlighter,
            modified: HashSet::new(),
            requested: None,
        }
    }

    fn join_lines(&mut self, first: usize, second: usize) -> (usize, usize) {
        styled_append(&mut self.styled[first], &self.plain[second]);
        self.styled.remove(second);

        let new_x = self.plain[first].chars().count();
        let text = self.plain.remove(second);
        self.plain[first].push_str(&text);

        self.modified.insert(first);
        self.modified.insert(second);

        (new_x, first)
    }

    fn split_lines(&mut self, x: usize, y: usize) {
        let i = y.saturating_add(1);
        if i < self.plain.len() {
            self.plain.insert(i, self.plain[y][x..].to_string());
            self.styled.insert(i, styled_split(&self.styled[y], x));
        } else {
            self.plain.push(self.plain[y][x..].to_string());
            self.styled.push(styled_split(&self.styled[y], x));
        }

        self.plain[y].truncate(x);
        styled_truncate(&mut self.styled[y], x);

        self.modified.insert(y);
        self.modified.insert(i);
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

    fn line_size(&self, line_no: usize) -> usize {
        self.plain.get(line_no).map(|l| l.chars().count()).unwrap_or_default()
    }

    fn is_editable(&self) -> bool {
        true
    }

    fn insert_char(&mut self, x: usize, y: usize, character: char) {
        if let Some(r) = get_byte_position(&self.plain, x, y) {
            if character == '\n' {
                if r.x.index == 0 {
                    self.plain.insert(y, String::new());
                    self.styled.insert(y, Vec::new());
                    self.modified.insert(y);
                } else {
                    self.split_lines(r.x.index, y);
                }
            } else {
                self.plain[y].insert(r.x.index, character);
                styled_insert(&mut self.styled[y], r.x.index, character);
                self.modified.insert(y);
            }
        } else if y < self.plain.len() {
            if character == '\n' {
                let i = y.saturating_add(1);
                if i < self.plain.len() {
                    self.plain.insert(i, String::new());
                    self.styled.insert(i, Vec::new());
                } else {
                    self.plain.push(String::new());
                    self.styled.push(Vec::new());
                }

                self.modified.insert(i);
            } else {
                self.plain[y].push(character);
                styled_push(&mut self.styled[y], character);
                self.modified.insert(y);
            }
        }
    }

    fn remove_char(&mut self, x: usize, y: usize, is_backspace: bool) -> Option<(usize, usize)> {
        if is_backspace && x == 0 && y > 0 && y < self.plain.len() {
            return Some(self.join_lines(y - 1, y));
        }

        if let Some(r) = get_byte_position(&self.plain, x, y) {
            let x = if is_backspace { r.x_prev } else { r.x };

            self.plain[y].remove(x.index);
            styled_remove(&mut self.styled[y], x.index);
            self.modified.insert(y);

            Some((x.char, y))
        } else if y < self.plain.len() {
            let x = if is_backspace { x.saturating_sub(1) } else { x };
            if let Some(r) = get_byte_position(&self.plain, x, y) {
                self.plain[y].remove(r.x.index);

                styled_remove(&mut self.styled[y], r.x.index);
                self.modified.insert(y);

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

fn styled_insert(line: &mut StyledLine, x: usize, c: char) {
    let mut current = 0;
    for part in line {
        if current + part.1.len() >= x {
            part.1.insert(x - current, c);
            return;
        }

        current += part.1.len();
    }
}

fn styled_append(line: &mut StyledLine, text: &str) {
    if let Some(part) = line.last_mut() {
        part.1.push_str(text);
    } else {
        line.push((Style::default(), text.to_owned()));
    }
}

fn styled_push(line: &mut StyledLine, c: char) {
    if let Some(part) = line.last_mut() {
        part.1.push(c);
    } else {
        line.push((Style::default(), c.to_string()));
    }
}

fn styled_remove(line: &mut StyledLine, x: usize) {
    let mut current = 0;
    for part in line {
        if current + part.1.len() > x {
            part.1.remove(x - current);
            return;
        }

        current += part.1.len();
    }
}

/// Splits [`StyledLine`] at `index` and returns second part.
fn styled_split(line: &StyledLine, index: usize) -> StyledLine {
    let mut result = Vec::new();
    let mut current = 0;
    let mut is_found = false;
    for part in line {
        if is_found {
            result.push((part.0, part.1.clone()));
        } else if current + part.1.len() > index {
            result.push((part.0, part.1[index - current..].to_string()));
            is_found = true;
        }

        current += part.1.len();
    }

    result
}

fn styled_truncate(line: &mut StyledLine, new_len: usize) {
    let mut current = 0;
    let mut new_end = None;
    for (i, part) in line.iter_mut().enumerate() {
        if current + part.1.len() > new_len {
            part.1.truncate(new_len - current);
            new_end = Some(i + 1);
            break;
        }

        current += part.1.len();
    }

    if let Some(new_end) = new_end
        && new_end < line.len()
    {
        line.truncate(new_end);
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
    pub x_next: Option<CharPosition>,
}

fn get_byte_position(text: &[String], x: usize, y: usize) -> Option<PositionSet> {
    if y < text.len() {
        let mut result = PositionSet::default();

        let mut found = false;
        for (i, (j, _)) in text[y].char_indices().enumerate() {
            if x > 0 && i == x - 1 {
                result.x_prev = CharPosition { char: i, index: j };
            }

            if i == x {
                result.x = CharPosition { char: i, index: j };
                found = true;
            }

            if i == x + 1 {
                result.x_next = Some(CharPosition { char: i, index: j });
                break;
            }
        }

        if found { Some(result) } else { None }
    } else {
        None
    }
}
