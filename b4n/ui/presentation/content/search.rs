use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// Represents a position in the content using `x` (column) and `y` (line) coordinates.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ContentPosition {
    pub x: usize,
    pub y: usize,
}

impl ContentPosition {
    /// Creates new [`ContentPosition`] instance.
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

#[derive(Default)]
pub struct SearchData {
    pub pattern: Option<String>,
    pub matches: Option<Vec<MatchPosition>>,
    pub current: Option<usize>,
}

/// Represents a match position in the content using `x` (column) and `y` (line) coordinates and the match length.
pub struct MatchPosition {
    pub x: usize,
    pub y: usize,
    pub length: usize,
}

impl MatchPosition {
    /// Creates new [`MatchPosition`] instance.
    pub fn new(x: usize, y: usize, length: usize) -> Self {
        Self { x, y, length }
    }

    /// Returns a new [`MatchPosition`] with its `x` and `y` coordinates offset by the given amount.
    pub fn adjust_by(&self, offset: Position) -> Self {
        Self {
            x: self.x.saturating_add(usize::from(offset.x)),
            y: self.y.saturating_add(usize::from(offset.y)),
            length: self.length,
        }
    }
}

/// Returns an appropriate search message based on the search direction.
pub fn get_search_wrapped_message(forward: bool) -> &'static str {
    if forward {
        "Reached end of search results"
    } else {
        "Reached start of search results"
    }
}

/// Widget that highlights search matches on the provided area.
pub struct SearchResultsWidget<'a> {
    /// Content's page position.
    page_start: ContentPosition,

    /// Search data.
    data: &'a SearchData,

    /// Highlight color.
    color: Color,

    /// Offset of the matches in the content.
    offset: Option<Position>,
}

impl<'a> SearchResultsWidget<'a> {
    pub fn new(page_start: ContentPosition, data: &'a SearchData, color: Color) -> Self {
        Self {
            page_start,
            data,
            color,
            offset: None,
        }
    }

    pub fn with_offset(mut self, offset: Option<Position>) -> Self {
        self.offset = offset;
        self
    }
}

impl Widget for SearchResultsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let Some(matches) = self.data.matches.as_deref() else {
            return;
        };

        if let Some(current) = self.data.current {
            self.highlight_match(area, buf, &matches[current.saturating_sub(1)]);
        } else {
            for m in matches {
                self.highlight_match(area, buf, m);
            }
        }
    }
}

impl SearchResultsWidget<'_> {
    fn highlight_match(&self, area: Rect, buf: &mut Buffer, position: &MatchPosition) {
        let m = if let Some(offset) = self.offset {
            &position.adjust_by(offset)
        } else {
            position
        };

        if m.y >= self.page_start.y && m.x.saturating_add(m.length) > self.page_start.x {
            self.highlight_cells(area, buf, m);
        }
    }

    fn highlight_cells(&self, area: Rect, buf: &mut Buffer, m: &MatchPosition) {
        let y = u16::try_from(m.y.saturating_sub(self.page_start.y)).unwrap_or_default();
        let mut length = m.length;

        while length > 0 {
            let x = u16::try_from(m.x.saturating_add(length).saturating_sub(self.page_start.x)).unwrap_or_default();
            length -= 1;

            let position = Position::new(x.saturating_add(area.x).saturating_sub(1), y.saturating_add(area.y));
            if area.contains(position)
                && let Some(cell) = buf.cell_mut(position)
            {
                cell.bg = self.color;
            }
        }
    }
}
