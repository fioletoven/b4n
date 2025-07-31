use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::Color,
};

#[derive(Default)]
pub struct SearchData {
    pub pattern: Option<String>,
    pub matches: Option<Vec<MatchPosition>>,
    pub current: Option<usize>,
}

/// Describes a match in the content.
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
pub fn highlight_search_matches(frame: &mut Frame<'_>, x: usize, y: usize, data: &SearchData, area: Rect, color: Color) {
    let Some(matches) = data.matches.as_deref() else {
        return;
    };

    if let Some(current) = data.current {
        highlight_match(frame, x, y, &matches[current.saturating_sub(1)], area, color);
    } else {
        for m in matches.iter().filter(|m| m.y >= y && m.x.saturating_add(m.length) > x) {
            highlight_match(frame, x, y, m, area, color);
        }
    }
}

fn highlight_match(frame: &mut Frame<'_>, x: usize, y: usize, m: &MatchPosition, area: Rect, color: Color) {
    let y = u16::try_from(m.y.saturating_sub(y)).unwrap_or_default();
    let mut length = m.length;

    while length > 0 {
        let x = u16::try_from(m.x.saturating_add(length).saturating_sub(x)).unwrap_or_default();
        length -= 1;

        let position = Position::new(x.saturating_add(area.x).saturating_sub(1), y.saturating_add(area.y));
        if area.contains(position) {
            if let Some(cell) = frame.buffer_mut().cell_mut(position) {
                cell.bg = color;
            }
        }
    }
}
