use ratatui::{
    layout::{Position, Rect},
    style::Color,
    widgets::Widget,
};

#[derive(Default)]
pub struct SearchData {
    pub pattern: Option<String>,
    pub matches: Option<Vec<MatchPosition>>,
    pub current: Option<usize>,
}

/// Describes a match position in the content.
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

/// Content page position.
#[derive(Default, Clone, Copy)]
pub struct PagePosition {
    pub x: usize,
    pub y: usize,
}

/// Widget that highlights search matches on the provided area.
pub struct SearchResultsWidget<'a> {
    /// Content's page position.
    position: PagePosition,

    /// Search data.
    data: &'a SearchData,

    /// Highlight color.
    color: Color,

    /// Offset of the matches in the content.
    offset: Option<Position>,
}

impl<'a> SearchResultsWidget<'a> {
    pub fn new(position: PagePosition, data: &'a SearchData, color: Color) -> Self {
        Self {
            position,
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
                highlight_match(area, buf, &self.position, r#match, self.color);
            } else {
                highlight_match(area, buf, &self.position, &matches[current.saturating_sub(1)], self.color);
            }
        } else {
            for m in matches {
                let m = if let Some(offset) = self.offset {
                    &m.adjust_by(offset)
                } else {
                    m
                };
                if m.y >= self.position.y && m.x.saturating_add(m.length) > self.position.x {
                    highlight_match(area, buf, &self.position, m, self.color);
                }
            }
        }
    }
}

fn highlight_match(area: Rect, buf: &mut ratatui::prelude::Buffer, position: &PagePosition, m: &MatchPosition, color: Color) {
    let y = u16::try_from(m.y.saturating_sub(position.y)).unwrap_or_default();
    let mut length = m.length;

    while length > 0 {
        let x = u16::try_from(m.x.saturating_add(length).saturating_sub(position.x)).unwrap_or_default();
        length -= 1;

        let position = Position::new(x.saturating_add(area.x).saturating_sub(1), y.saturating_add(area.y));
        if area.contains(position)
            && let Some(cell) = buf.cell_mut(position)
        {
            cell.bg = color;
        }
    }
}
