use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::Color,
};

/// Contains match position in content.
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
}

/// Highlights all search matches within the specified area, adjusted by the given scroll offsets `x` and `y`.
///
/// ## Parameters
/// - `frame`: The frame to render the highlights on.
/// - `x`: Horizontal scroll offset.
/// - `y`: Vertical scroll offset.
/// - `matches`: Optional list of match positions to highlight.
/// - `area`: The rectangular area within which to highlight matches.
/// - `color`: The color used to highlight the matches.
pub fn highlight_search_matches(
    frame: &mut Frame<'_>,
    x: usize,
    y: usize,
    matches: Option<Vec<MatchPosition>>,
    area: Rect,
    color: Color,
) {
    let Some(matches) = matches else {
        return;
    };

    for m in matches {
        let y = u16::try_from(m.y.saturating_sub(y)).unwrap_or_default();
        let mut length = m.length;

        while y > 0 && length > 0 {
            let x = u16::try_from(m.x.saturating_add(length).saturating_sub(x)).unwrap_or_default();
            length -= 1;

            let position = Position::new(x, y);
            if area.contains(position) {
                if let Some(cell) = frame.buffer_mut().cell_mut(position) {
                    cell.bg = color;
                }
            }
        }
    }
}
