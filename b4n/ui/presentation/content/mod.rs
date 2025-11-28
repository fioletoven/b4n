pub use header::ContentHeader;
pub use search::{ContentPosition, MatchPosition};
pub use select::Selection;
pub use styled_line::{StyleFallback, StyledLine, StyledLineExt, VecStyledLineExt};
pub use viewer::ContentViewer;

mod edit;
mod header;
mod search;
mod select;
mod styled_line;
mod viewer;

use b4n_tui::ResponseEvent;

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

    /// Converts the content to a plain `String` representation, optionally restricting the range of the content to be converted.
    fn to_plain_text(&self, range: Option<Selection>) -> String;

    /// Searches content for the specified pattern.
    fn search(&self, pattern: &str) -> Vec<MatchPosition>;

    /// Returns characters count for the longest line in the content.
    fn max_size(&self) -> usize;

    /// Returns text of the line under `line_no` index.
    fn line(&self, line_no: usize) -> Option<&str> {
        let _ = line_no;
        None
    }

    /// Returns characters count of the line under `line_no` index.
    fn line_size(&self, line_no: usize) -> usize;

    /// Returns max vertical start of the page for the specified height.
    fn max_vstart(&self, page_height: u16) -> usize {
        self.len().saturating_sub(page_height.into())
    }

    /// Returns max horizontal start of the page for the specified width.
    fn max_hstart(&self, page_width: u16) -> usize {
        self.max_size().saturating_sub(page_width.into())
    }

    /// Returns `true` if content can be edited.
    fn is_editable(&self) -> bool {
        false
    }

    /// Returns the number of leading spaces in the line at the given `line_no`.
    fn leading_spaces(&self, line_no: usize) -> Option<usize> {
        if line_no < self.len() { Some(0) } else { None }
    }

    /// Returns the start and end (char indices) of the word that contains the character at `idx` for the specified `line_no`.
    fn word_bounds(&self, position: ContentPosition) -> Option<(usize, usize)>;

    /// Inserts specified character to the content at a position `x:y`.
    fn insert_char(&mut self, position: ContentPosition, character: char) {
        let _ = position;
        let _ = character;
    }

    /// Inserts specified text at a position `x:y`.
    fn insert_text(&mut self, position: ContentPosition, text: Vec<String>) {
        let _ = position;
        let _ = text;
    }

    /// Deletes character at a position `x:y`.\
    /// **Note** that it returns a new position.
    fn remove_char(&mut self, position: ContentPosition, is_backspace: bool) -> Option<ContentPosition> {
        let _ = position;
        let _ = is_backspace;
        None
    }

    /// Removes the specified `range` from the content.
    fn remove_text(&mut self, range: Selection) {
        let _ = range;
    }

    /// Reverts most recent changes done in edit mode.
    fn undo(&mut self) -> Option<ContentPosition> {
        None
    }

    /// Re-applies an action that was previously undone.
    fn redo(&mut self) -> Option<ContentPosition> {
        None
    }

    /// Can be called on every app tick to do some computation.
    fn process_tick(&mut self) -> ResponseEvent {
        ResponseEvent::Handled
    }
}
