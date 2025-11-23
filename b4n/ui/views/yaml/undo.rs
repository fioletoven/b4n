use std::time::{Duration, Instant};

use crate::ui::presentation::{ContentPosition, Selection};

pub enum UndoMode {
    Insert,
    Remove,
    Cut,
    Paste,
}

pub struct Undo {
    pub pos: ContentPosition,
    pub end: Option<ContentPosition>,
    pub ch: char,
    pub text: Option<Vec<String>>,
    pub mode: UndoMode,
    pub when: Instant,
}

impl Undo {
    pub fn insert(pos: ContentPosition, ch: char) -> Self {
        Self {
            pos,
            end: None,
            ch,
            text: None,
            mode: UndoMode::Insert,
            when: Instant::now(),
        }
    }

    pub fn remove(pos: ContentPosition, ch: char) -> Self {
        Self {
            pos,
            end: None,
            ch,
            text: None,
            mode: UndoMode::Remove,
            when: Instant::now(),
        }
    }

    pub fn cut(range: Selection, removed_text: Vec<String>) -> Self {
        Self {
            pos: range.start,
            end: Some(range.end),
            ch: ' ',
            text: Some(removed_text),
            mode: UndoMode::Cut,
            when: Instant::now(),
        }
    }
}

pub fn pop_recent_group(vec: &mut Vec<Undo>, threshold: Duration) -> Vec<Undo> {
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
