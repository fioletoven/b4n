use b4n_tui::{MouseEventKind, TuiEvent};
use ratatui::{
    layout::{Position, Rect},
    widgets::Widget,
};

use crate::ui::presentation::{Content, content::search::PagePosition};

/// Context for the selected text.
#[derive(Default)]
pub struct SelectContext {
    start: Option<PagePosition>,
    end: Option<PagePosition>,
}

impl SelectContext {
    /// Process UI key/mouse event.
    pub fn process_event<T: Content>(&mut self, event: &TuiEvent, content: &mut T, page_start: &mut PagePosition, area: Rect) {
        let TuiEvent::Mouse(mouse) = event else {
            return;
        };

        if mouse.kind == MouseEventKind::LeftClick {
            let pos = get_position_in_content(area, *page_start, mouse.column, mouse.row);
            self.start = Some(pos);
            self.end = None;
            return;
        }

        if mouse.kind == MouseEventKind::LeftDrag {
            scroll_page_if_needed(area, page_start, content, mouse.column, mouse.row);
            let pos = get_position_in_content(area, *page_start, mouse.column, mouse.row);
            self.end = Some(pos);
        }
    }
}

fn scroll_page_if_needed<T: Content>(area: Rect, page_start: &mut PagePosition, content: &mut T, mouse_x: u16, mouse_y: u16) {
    // scroll page vertically while dragging
    if mouse_y > (area.y + area.height).saturating_sub(5) {
        page_start.y += 3;
    } else if mouse_y < area.y + 5 {
        page_start.y = page_start.y.saturating_sub(3)
    }

    // scroll page horizontally while dragging
    if mouse_x > (area.x + area.width).saturating_sub(5) {
        page_start.x += 3;
    } else if mouse_x < area.x + 5 {
        page_start.x = page_start.x.saturating_sub(3)
    }

    // apply page start constraints
    if page_start.y > content.max_vstart(area.height) {
        page_start.y = content.max_vstart(area.height);
    }

    if page_start.x > content.max_hstart(area.width) {
        page_start.x = content.max_hstart(area.width);
    }
}

fn get_position_in_content(area: Rect, page_start: PagePosition, mouse_x: u16, mouse_y: u16) -> PagePosition {
    let x = page_start.x.saturating_add(mouse_x.saturating_sub(area.x).into());
    let y = page_start.y.saturating_add(mouse_y.saturating_sub(area.y).into());
    PagePosition { x, y }
}

/// Widget that draws selection on the content.
pub struct ContentSelectWidget<'a> {
    pub context: &'a SelectContext,
    pub page_start: &'a PagePosition,
    pub page_area: &'a Rect,
}

impl<'a> ContentSelectWidget<'a> {
    /// Creates new [`ContentSelectWidget`] instance.
    pub fn new(context: &'a SelectContext, page_start: &'a PagePosition, page_area: &'a Rect) -> Self {
        Self {
            context,
            page_start,
            page_area,
        }
    }

    fn get_relative_position(&self, position: PagePosition, area: Rect) -> Position {
        let x = if position.x >= self.page_start.x {
            u16::try_from(position.x - self.page_start.x).unwrap_or(self.page_area.width)
        } else {
            self.page_area.width
        };

        let y = if position.y >= self.page_start.y {
            u16::try_from(position.y - self.page_start.y).unwrap_or(self.page_area.height)
        } else {
            self.page_area.height
        };

        Position {
            x: x.saturating_add(area.x),
            y: y.saturating_add(area.y),
        }
    }
}

impl Widget for ContentSelectWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if let Some(start) = self.context.start
            && let Some(end) = self.context.end
        {
            let (start, end) = sort(start, end);
            let start = self.get_relative_position(start, area);
            let end = self.get_relative_position(end, area);

            if area.contains(start)
                && let Some(cell) = buf.cell_mut(start)
            {
                cell.bg = ratatui::style::Color::Cyan;
            }

            if area.contains(end)
                && let Some(cell) = buf.cell_mut(end)
            {
                cell.bg = ratatui::style::Color::Red;
            }
        }
    }
}

fn sort(p1: PagePosition, p2: PagePosition) -> (PagePosition, PagePosition) {
    if (p1.y > p2.y) || (p1.y == p2.y && p1.x > p2.x) {
        (p2, p1)
    } else {
        (p1, p2)
    }
}
