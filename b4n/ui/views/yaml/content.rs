use b4n_common::{slice_from, slice_to, substring};
use b4n_tasks::{HighlightError, HighlightRequest, HighlightResponse};
use b4n_tui::ResponseEvent;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc::UnboundedSender, oneshot::Receiver};

use crate::ui::presentation::utils::VecStringExt;
use crate::ui::presentation::{Content, MatchPosition, Selection, StyleFallback, StyledLine, StyledLineExt, VecStyledLineExt};

/// Number of lines before and after the modified section to include in the re-highlighting process.
const HIGHLIGHT_CONTEXT_LINES_NO: usize = 200;

/// Styled YAML content.
pub struct YamlContent {
    pub styled: Vec<StyledLine>,
    pub plain: Vec<String>,
    pub lowercase: Vec<String>,
    max_size: usize,
    max_line_no: usize,
    highlighter: UnboundedSender<HighlightRequest>,
    requested: Option<RequestedHighlight>,
    is_editable: bool,
    modified: HashSet<usize>,
    undo: Vec<Undo>,
    redo: Vec<Vec<Undo>>,
    fallback: StyleFallback,
}

impl YamlContent {
    /// Creates new [`YamlContent`] instance.
    pub fn new(
        styled: Vec<StyledLine>,
        plain: Vec<String>,
        highlighter: UnboundedSender<HighlightRequest>,
        is_editable: bool,
        fallback: StyleFallback,
    ) -> Self {
        let (max_line_no, max_size) = get_longest_line(&plain);
        let lowercase = plain.iter().map(|l| l.to_ascii_lowercase()).collect();

        Self {
            styled,
            plain,
            lowercase,
            max_size,
            max_line_no,
            highlighter,
            requested: None,
            is_editable,
            modified: HashSet::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            fallback,
        }
    }

    fn mark_line_as_modified(&mut self, line_no: usize) {
        if let Some(line) = self.plain.get(line_no) {
            self.modified.insert(line_no);
            let len = line.chars().count();
            if len > self.max_size {
                self.max_size = len;
                self.max_line_no = line_no;
            } else if line_no == self.max_line_no {
                (self.max_line_no, self.max_size) = get_longest_line(&self.plain);
            }
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

    fn join_lines(&mut self, line_no: usize) -> (usize, usize) {
        let new_x = self.plain[line_no].chars().count();

        self.styled.join_lines(line_no);
        self.plain.join_lines(line_no);
        self.lowercase.join_lines(line_no);

        self.mark_line_as_modified(line_no);
        self.mark_line_as_modified(line_no + 1);

        (new_x, line_no)
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

    fn insert_char_internal(&mut self, x: usize, y: usize, ch: char) {
        if let Some(r) = get_char_position(&self.plain, x, y) {
            if ch == '\n' {
                if r.x.index == 0 {
                    self.add_empty_line(y);
                } else {
                    self.split_lines(r.x.index, y);
                }
            } else {
                self.plain[y].insert(r.x.index, ch);
                self.lowercase[y].insert(r.x.index, ch.to_ascii_lowercase());
                self.styled[y].sl_insert(r.x.index, ch);
                self.mark_line_as_modified(y);
            }
        } else if y < self.plain.len() {
            if ch == '\n' {
                self.add_empty_line(y + 1);
            } else {
                self.plain[y].push(ch);
                self.lowercase[y].push(ch.to_ascii_lowercase());
                self.styled[y].sl_push(ch, &self.fallback);
                self.mark_line_as_modified(y);
            }
        }
    }

    fn remove_char_internal(&mut self, x: usize, y: usize, is_backspace: bool, track_undo: bool) -> Option<(usize, usize)> {
        if is_backspace && x == 0 {
            if y > 0 && y < self.plain.len() {
                let (x, y) = self.join_lines(y - 1);
                return Some(self.track_remove(x, y, '\n', track_undo));
            }

            return Some((x, y));
        }

        if let Some(r) = get_char_position(&self.plain, x, y) {
            let x = if is_backspace { r.x_prev } else { r.x };
            let ch = self.remove_ch(x.index, y);
            Some(self.track_remove(x.char, y, ch, track_undo))
        } else if y < self.plain.len() {
            let x = if is_backspace { x.saturating_sub(1) } else { x };
            if let Some(r) = get_char_position(&self.plain, x, y) {
                let ch = self.remove_ch(r.x.index, y);
                Some(self.track_remove(r.x.char, y, ch, track_undo))
            } else if y + 1 < self.plain.len() {
                let (x, y) = self.join_lines(y);
                Some(self.track_remove(x, y, '\n', track_undo))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn remove_ch(&mut self, idx: usize, line_no: usize) -> char {
        let removed = self.plain[line_no].remove(idx);
        self.lowercase[line_no].remove(idx);
        self.styled[line_no].sl_remove(idx);

        self.mark_line_as_modified(line_no);
        removed
    }

    fn track_remove(&mut self, x: usize, y: usize, ch: char, track: bool) -> (usize, usize) {
        if track {
            self.undo.push(Undo::remove(x, y, ch));
        }

        (x, y)
    }

    fn remove_text_internal(&mut self, range: Selection) -> Vec<String> {
        self.styled.remove_text(range.clone());
        self.lowercase.remove_text(range.clone());
        self.plain.remove_text(range)
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

    fn to_plain_text(&self, range: Option<Selection>) -> String {
        match range.map(|r| r.sorted()) {
            None => self.plain.join("\n"),
            Some((start, end)) => {
                let start_line = start.y.min(self.plain.len().saturating_sub(1));
                let end_line = end.y.min(self.plain.len().saturating_sub(1));

                let mut result = String::new();
                for i in start_line..=end_line {
                    let line = &self.plain[i];
                    if i == start_line && i == end_line {
                        result.push_str(substring(line, start.x, (end.x + 1).saturating_sub(start.x)));
                        if line.chars().count() < end.x + 1 {
                            result.push('\n');
                        }
                    } else if i == start_line {
                        result.push_str(slice_from(line, start.x));
                        result.push('\n');
                    } else if i == end_line {
                        result.push_str(slice_to(line, end.x + 1));
                        if line.chars().count() < end.x + 1 {
                            result.push('\n');
                        }
                    } else {
                        result.push_str(line);
                        result.push('\n');
                    }
                }

                result
            },
        }
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

    fn leading_spaces(&self, line_no: usize) -> Option<usize> {
        self.plain
            .get(line_no)
            .map(|line| line.chars().take_while(|c| *c == ' ').count())
    }

    fn word_bounds(&self, line_no: usize, idx: usize) -> Option<(usize, usize)> {
        if line_no < self.plain.len() {
            b4n_common::word_bounds(&self.plain[line_no], idx)
        } else {
            None
        }
    }

    fn insert_char(&mut self, x: usize, y: usize, ch: char) {
        self.redo.clear();
        self.undo.push(Undo::insert(x, y, ch));
        self.insert_char_internal(x, y, ch);
    }

    fn remove_char(&mut self, x: usize, y: usize, is_backspace: bool) -> Option<(usize, usize)> {
        self.redo.clear();
        self.remove_char_internal(x, y, is_backspace, true)
    }

    fn remove_text(&mut self, range: Selection) {
        self.mark_line_as_modified(range.start.y);
        self.mark_line_as_modified(range.end.y);
        let removed = self.remove_text_internal(range);
    }

    fn undo(&mut self) -> Option<(usize, usize)> {
        let actions = pop_recent_group(&mut self.undo, Duration::from_millis(300));
        if actions.is_empty() {
            None
        } else {
            let mut result = None;
            for action in &actions {
                if action.is_insert {
                    self.remove_char_internal(action.x, action.y, false, false);
                    result = Some((action.x, action.y));
                } else {
                    self.insert_char_internal(action.x, action.y, action.ch);
                    if action.ch == '\n' {
                        result = Some((0, action.y.saturating_add(1)));
                    } else {
                        result = Some((action.x.saturating_add(1), action.y));
                    }
                }
            }

            self.redo.push(actions);
            result
        }
    }

    fn redo(&mut self) -> Option<(usize, usize)> {
        if let Some(mut actions) = self.redo.pop() {
            let mut result = None;

            actions.reverse();
            for action in &actions {
                if action.is_insert {
                    self.insert_char_internal(action.x, action.y, action.ch);
                    if action.ch == '\n' {
                        result = Some((0, action.y.saturating_add(1)));
                    } else {
                        result = Some((action.x.saturating_add(1), action.y));
                    }
                } else {
                    self.remove_char_internal(action.x, action.y, false, false);
                    result = Some((action.x, action.y));
                }
            }

            self.undo.extend(actions);
            result
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
                // there are no new modifications, we can apply the styled fragment
                self.styled.splice(requested.start..=requested.end, response.styled);
            } else {
                // there are new modifications, we need to rollback modified lines, as the styled fragment is outdated
                self.modified.extend(requested.first..=requested.last);
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
                first,
                last,
                response: rx,
            });
        }

        ResponseEvent::Handled
    }
}

struct RequestedHighlight {
    pub start: usize,
    pub end: usize,
    pub first: usize,
    pub last: usize,
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

struct Undo {
    x: usize,
    y: usize,
    ch: char,
    is_insert: bool,
    when: Instant,
}

impl Undo {
    fn insert(x: usize, y: usize, ch: char) -> Self {
        Self {
            x,
            y,
            ch,
            is_insert: true,
            when: Instant::now(),
        }
    }

    fn remove(x: usize, y: usize, ch: char) -> Self {
        Self {
            x,
            y,
            ch,
            is_insert: false,
            when: Instant::now(),
        }
    }
}

fn pop_recent_group(vec: &mut Vec<Undo>, threshold: Duration) -> Vec<Undo> {
    let mut group = Vec::new();

    if let Some(last) = vec.pop() {
        let mut reference_time = last.when;
        group.push(last);

        while let Some(peek) = vec.last() {
            if reference_time.duration_since(peek.when) <= threshold {
                let action = vec.pop().unwrap();
                reference_time = action.when;
                group.push(action);
            } else {
                break;
            }
        }
    }

    group
}

fn get_longest_line(plain: &[String]) -> (usize, usize) {
    plain
        .iter()
        .enumerate()
        .map(|(i, l)| (i, l.chars().count()))
        .max_by_key(|&(_, count)| count)
        .unwrap_or((0, 0))
}
