use b4n_common::{slice_from, slice_to, substring};
use b4n_tasks::{HighlightError, HighlightRequest, HighlightResponse};
use b4n_tui::ResponseEvent;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;
use tokio::sync::{mpsc::UnboundedSender, oneshot::Receiver};

use crate::ui::presentation::utils::{VecStringExt, get_char_position};
use crate::ui::presentation::{
    Content, ContentPosition, MatchPosition, Selection, StyleFallback, StyledLine, StyledLineExt, VecStyledLineExt,
};
use crate::ui::views::yaml::undo::{Undo, UndoMode, pop_recent_group};

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

    fn join_lines(&mut self, line_no: usize) -> ContentPosition {
        let new_x = self.plain[line_no].chars().count();

        self.styled.join_lines(line_no);
        self.plain.join_lines(line_no);
        self.lowercase.join_lines(line_no);

        self.mark_line_as_modified(line_no);
        self.mark_line_as_modified(line_no + 1);

        ContentPosition::new(new_x, line_no)
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

    fn insert_char_internal(&mut self, pos: ContentPosition, ch: char) {
        if let Some(r) = get_char_position(&self.plain, pos) {
            if ch == '\n' {
                if r.x.index == 0 {
                    self.add_empty_line(pos.y);
                } else {
                    self.split_lines(r.x.index, pos.y);
                }
            } else {
                self.plain[pos.y].insert(r.x.index, ch);
                self.lowercase[pos.y].insert(r.x.index, ch.to_ascii_lowercase());
                self.styled[pos.y].sl_insert(r.x.index, ch);
                self.mark_line_as_modified(pos.y);
            }
        } else if pos.y < self.plain.len() {
            if ch == '\n' {
                self.add_empty_line(pos.y + 1);
            } else {
                self.plain[pos.y].push(ch);
                self.lowercase[pos.y].push(ch.to_ascii_lowercase());
                self.styled[pos.y].sl_push(ch, &self.fallback);
                self.mark_line_as_modified(pos.y);
            }
        }
    }

    fn remove_char_internal(&mut self, pos: ContentPosition, is_backspace: bool, track_undo: bool) -> Option<ContentPosition> {
        if is_backspace && pos.x == 0 {
            if pos.y > 0 && pos.y < self.plain.len() {
                let new_pos = self.join_lines(pos.y - 1);
                return Some(self.track_remove(new_pos, '\n', track_undo));
            }

            return Some(pos);
        }

        if let Some(r) = get_char_position(&self.plain, pos) {
            let x = if is_backspace { r.x_prev } else { r.x };
            let ch = self.remove_ch(x.index, pos.y);
            Some(self.track_remove(ContentPosition::new(x.char, pos.y), ch, track_undo))
        } else if pos.y < self.plain.len() {
            let x = if is_backspace { pos.x.saturating_sub(1) } else { pos.x };
            if let Some(r) = get_char_position(&self.plain, ContentPosition::new(x, pos.y)) {
                let ch = self.remove_ch(r.x.index, pos.y);
                Some(self.track_remove(ContentPosition::new(r.x.char, pos.y), ch, track_undo))
            } else if pos.y + 1 < self.plain.len() {
                let new_pos = self.join_lines(pos.y);
                Some(self.track_remove(new_pos, '\n', track_undo))
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

    fn track_remove(&mut self, pos: ContentPosition, ch: char, track: bool) -> ContentPosition {
        if track {
            self.undo.push(Undo::remove(pos, ch));
        }

        pos
    }

    fn remove_text_internal(&mut self, range: &Selection) -> Vec<String> {
        self.styled.remove_text(range);
        self.lowercase.remove_text(range);
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

    fn word_bounds(&self, position: ContentPosition) -> Option<(usize, usize)> {
        if position.y < self.plain.len() {
            b4n_common::word_bounds(&self.plain[position.y], position.x)
        } else {
            None
        }
    }

    fn insert_char(&mut self, position: ContentPosition, ch: char) {
        self.redo.clear();
        self.undo.push(Undo::insert(position, ch));
        self.insert_char_internal(position, ch);
    }

    fn remove_char(&mut self, position: ContentPosition, is_backspace: bool) -> Option<ContentPosition> {
        self.redo.clear();
        self.remove_char_internal(position, is_backspace, true)
    }

    fn remove_text(&mut self, range: Selection) {
        self.mark_line_as_modified(range.start.y);
        self.mark_line_as_modified(range.end.y);
        let removed = self.remove_text_internal(&range);
        self.redo.clear();
        self.undo.push(Undo::cut(range, removed));
    }

    fn insert_text(&mut self, position: ContentPosition, text: Vec<String>) {
        self.mark_line_as_modified(position.y);
        self.mark_line_as_modified(position.y + text.len());
        self.styled.insert_text(position, &text, &self.fallback);
        self.lowercase
            .insert_text(position, text.iter().map(|s| s.to_lowercase()).collect());
        self.plain.insert_text(position, text);
    }

    fn undo(&mut self) -> Option<ContentPosition> {
        let mut actions = pop_recent_group(&mut self.undo, Duration::from_millis(300));
        if actions.is_empty() {
            return None;
        }

        let mut result = None;
        for action in &mut actions {
            match action.mode {
                UndoMode::Insert => {
                    self.remove_char_internal(action.pos, false, false);
                    result = Some(action.pos);
                },
                UndoMode::Remove => {
                    self.insert_char_internal(action.pos, action.ch);
                    if action.ch == '\n' {
                        result = Some(ContentPosition::new(0, action.pos.y.saturating_add(1)));
                    } else {
                        result = Some(ContentPosition::new(action.pos.x.saturating_add(1), action.pos.y));
                    }
                },
                UndoMode::Cut => {
                    if let Some(end) = action.end {
                        let text = action.text.take();
                        if let Some(text) = text {
                            self.insert_text(action.pos, text);
                            result = Some(ContentPosition { x: end.x + 1, y: end.y });
                        }
                    }
                },
            }
        }

        self.redo.push(actions);
        result
    }

    fn redo(&mut self) -> Option<ContentPosition> {
        let mut actions = self.redo.pop()?;
        let mut result = None;

        actions.reverse();
        for action in &mut actions {
            match action.mode {
                UndoMode::Insert => {
                    self.insert_char_internal(action.pos, action.ch);
                    if action.ch == '\n' {
                        result = Some(ContentPosition::new(0, action.pos.y.saturating_add(1)));
                    } else {
                        result = Some(ContentPosition::new(action.pos.x.saturating_add(1), action.pos.y));
                    }
                },
                UndoMode::Remove => {
                    self.remove_char_internal(action.pos, false, false);
                    result = Some(action.pos);
                },
                UndoMode::Cut => {
                    if let Some(end) = action.end {
                        let range = Selection::new(action.pos, end);
                        action.text = Some(self.remove_text_internal(&range));
                        result = Some(action.pos);
                    }
                },
            }
        }

        self.undo.extend(actions);
        result
    }

    fn process_tick(&mut self) -> ResponseEvent {
        if let Some(requested) = &mut self.requested
            && let Ok(response) = requested.response.try_recv()
        {
            if self.modified.is_empty()
                && let Ok(response) = response
            {
                // there are no new modifications, we can apply the styled fragment
                let start = requested.start.min(self.styled.len().saturating_sub(1));
                let end = requested.end.min(self.styled.len().saturating_sub(1));
                self.styled.splice(start..=end, response.styled);
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

fn get_longest_line(plain: &[String]) -> (usize, usize) {
    plain
        .iter()
        .enumerate()
        .map(|(i, l)| (i, l.chars().count()))
        .max_by_key(|&(_, count)| count)
        .unwrap_or((0, 0))
}
