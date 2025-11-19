use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// Represents a position in the content using `x` (column) and `y` (line) coordinates.
#[derive(Default, Clone, Copy)]
pub struct ContentPosition {
    pub x: usize,
    pub y: usize,
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
        "Reached end of search results."
    } else {
        "Reached start of search results."
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
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let Some(matches) = self.data.matches.as_deref() else {
            return;
        };

        if let Some(current) = self.data.current {
            if let Some(offset) = self.offset {
                let r#match = &matches[current.saturating_sub(1)].adjust_by(offset);
                highlight_match(area, buf, &self.page_start, r#match, self.color);
            } else {
                highlight_match(area, buf, &self.page_start, &matches[current.saturating_sub(1)], self.color);
            }
        } else {
            for m in matches {
                let m = if let Some(offset) = self.offset {
                    &m.adjust_by(offset)
                } else {
                    m
                };
                if m.y >= self.page_start.y && m.x.saturating_add(m.length) > self.page_start.x {
                    highlight_match(area, buf, &self.page_start, m, self.color);
                }
            }
        }
    }
}

fn highlight_match(
    area: Rect,
    buf: &mut ratatui::prelude::Buffer,
    page_start: &ContentPosition,
    m: &MatchPosition,
    color: Color,
) {
    let y = u16::try_from(m.y.saturating_sub(page_start.y)).unwrap_or_default();
    let mut length = m.length;

    while length > 0 {
        let x = u16::try_from(m.x.saturating_add(length).saturating_sub(page_start.x)).unwrap_or_default();
        length -= 1;

        let position = Position::new(x.saturating_add(area.x).saturating_sub(1), y.saturating_add(area.y));
        if area.contains(position)
            && let Some(cell) = buf.cell_mut(position)
        {
            cell.bg = color;
        }
    }
}
