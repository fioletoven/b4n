pub use header::ContentHeader;
pub use search::MatchPosition;
pub use styled_line::{StyledLine, StyledLineExt};
pub use viewer::ContentViewer;

mod edit;
mod header;
mod search;
mod styled_line;
mod viewer;

use crate::ui::ResponseEvent;

/// Content for the [`ContentViewer`].
pub trait Content {
    /// Returns page with [`StyledLine`]s.
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine];

    /// Returns the length of a [`Content`].
    fn len(&self) -> usize;

    /// Returns `true` if `self` has a length of zero lines.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a hash calculated over the content.
    fn hash(&self) -> u64;

    /// Searches content for the specified pattern.
    fn search(&self, pattern: &str) -> Vec<MatchPosition>;

    /// Returns characters count for the longest line in the content.
    fn max_size(&self) -> usize;

    /// Returns characters count of the line under `line_no` index.
    fn line_size(&self, line_no: usize) -> usize;

    /// Returns `true` if content can be edited.
    fn is_editable(&self) -> bool {
        false
    }

    /// Inserts specified character to the content at a position `x:y`.
    fn insert_char(&mut self, x: usize, y: usize, character: char) {
        let _ = x;
        let _ = y;
        let _ = character;
    }

    /// Deletes character at a position `x` and `y`.\
    /// **Note** that it returns a new position.
    fn remove_char(&mut self, x: usize, y: usize, is_backspace: bool) -> Option<(usize, usize)> {
        let _ = x;
        let _ = y;
        let _ = is_backspace;
        None
    }

    /// Reverts most recent changes done in edit mode.
    fn undo(&mut self) -> Option<(usize, usize)> {
        None
    }

    /// Re-applies an action that was previously undone.
    fn redo(&mut self) -> Option<(usize, usize)> {
        None
    }

    /// Can be called on every app tick to do some computation.
    fn process_tick(&mut self) -> ResponseEvent {
        ResponseEvent::Handled
    }
}
