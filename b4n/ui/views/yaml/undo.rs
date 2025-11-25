use std::time::{Duration, Instant};

use crate::ui::presentation::{ContentPosition, Selection};

/// Represents the type of change stored in an undo/redo entry.
pub enum UndoMode {
    Insert,
    Remove,
    Cut,
}

/// Stores a single undo/redo action.
pub struct Undo {
    pub pos: ContentPosition,
    pub end: Option<ContentPosition>,
    pub ch: char,
    pub text: Option<Vec<String>>,
    pub mode: UndoMode,
    pub when: Instant,
}

impl Undo {
    /// Creates a new undo/redo entry representing an inserted character.
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

    /// Creates a new undo/redo entry representing a removed character.
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

    /// Creates a new undo/redo entry representing a cut (range removal).
    pub fn cut(range: Selection, removed_text: Vec<String>) -> Self {
        let (start, end) = range.sorted();
        Self {
            pos: start,
            end: Some(end),
            ch: ' ',
            text: Some(removed_text),
            mode: UndoMode::Cut,
            when: Instant::now(),
        }
    }
}

/// Pops the most recent undo actions that occurred within the given `threshold` time window.\
/// This groups quick successive edits into a single undo step.
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
