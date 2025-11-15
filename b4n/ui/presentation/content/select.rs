use b4n_tui::{MouseEventKind, TuiEvent};
use ratatui::{layout::Rect, widgets::Widget};

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

        if mouse.kind == MouseEventKind::LeftDoubleClick {
            let pos = get_position_in_content(area, content, *page_start, mouse.column, mouse.row);
            if let Some((start, end)) = content.word_bounds(pos.y, pos.x) {
                self.start = Some(PagePosition { x: start, y: pos.y });
                self.end = Some(PagePosition { x: end, y: pos.y });
            }

            return;
        }

        if mouse.kind == MouseEventKind::LeftClick {
            let pos = get_position_in_content(area, content, *page_start, mouse.column, mouse.row);
            self.start = Some(pos);
            self.end = None;
            return;
        }

        if mouse.kind == MouseEventKind::LeftDrag {
            scroll_page_if_needed(area, page_start, content, mouse.column, mouse.row);
            let pos = get_position_in_content(area, content, *page_start, mouse.column, mouse.row);
            self.end = Some(pos);
        }
    }
}

fn scroll_page_if_needed<T: Content>(area: Rect, page_start: &mut PagePosition, content: &mut T, mouse_x: u16, mouse_y: u16) {
    // scroll page vertically while dragging
    if mouse_y > (area.y + area.height).saturating_sub(3) {
        page_start.y += 2;
    } else if mouse_y < area.y + 3 {
        page_start.y = page_start.y.saturating_sub(2)
    }

    // scroll page horizontally while dragging
    if mouse_x > (area.x + area.width).saturating_sub(3) {
        page_start.x += 2;
    } else if mouse_x < area.x + 3 {
        page_start.x = page_start.x.saturating_sub(2)
    }

    // apply page start constraints
    if page_start.y > content.max_vstart(area.height) {
        page_start.y = content.max_vstart(area.height);
    }

    if page_start.x > content.max_hstart(area.width) {
        page_start.x = content.max_hstart(area.width);
    }
}

fn get_position_in_content<T: Content>(
    area: Rect,
    content: &T,
    page_start: PagePosition,
    mouse_x: u16,
    mouse_y: u16,
) -> PagePosition {
    let x = page_start.x.saturating_add(mouse_x.saturating_sub(area.x).into());
    let y = page_start.y.saturating_add(mouse_y.saturating_sub(area.y).into());

    let x = x.min(content.line_size(y));
    PagePosition { x, y }
}

/// Widget that draws selection on the content.
pub struct ContentSelectWidget<'a, T: Content> {
    pub context: &'a SelectContext,
    pub content: &'a T,
    pub page_start: &'a PagePosition,
}

impl<'a, T: Content> ContentSelectWidget<'a, T> {
    /// Creates new [`ContentSelectWidget`] instance.
    pub fn new(context: &'a SelectContext, content: &'a T, page_start: &'a PagePosition) -> Self {
        Self {
            context,
            content,
            page_start,
        }
    }

    fn get_relative_x(&self, x: usize, area: Rect) -> Option<u16> {
        let x = x.checked_sub(self.page_start.x)?;
        let x = u16::try_from(x).unwrap_or(area.width);
        Some(x.saturating_add(area.x))
    }

    fn get_relative_y(&self, y: usize, area: Rect) -> Option<u16> {
        let y = y.checked_sub(self.page_start.y)?;
        let y = u16::try_from(y).unwrap_or(area.height);
        Some(y.saturating_add(area.y))
    }

    fn get_relative_max_len(&self, area: Rect, current_line: usize) -> Option<u16> {
        let line_len = self.content.line_size(current_line) + 1;
        let area_x = usize::from(area.x);

        if current_line >= self.content.len() || line_len < self.page_start.x + area_x {
            return None;
        }

        let max_x = line_len.checked_sub(self.page_start.x + area_x)? + 1;
        Some(u16::try_from(max_x).unwrap_or(u16::MAX))
    }
}

impl<'a, T: Content> Widget for ContentSelectWidget<'a, T> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let (Some(start), Some(end)) = (self.context.start, self.context.end) else {
            return;
        };
        let (start, end) = sort(start, end);

        for current_line in start.y..=end.y {
            if let Some(y) = self.get_relative_y(current_line, area)
                && y >= area.y
                && y < area.bottom()
                && let Some(max_x) = self.get_relative_max_len(area, current_line)
            {
                let start = if start.y == current_line {
                    // if this is the first line in the selection
                    self.get_relative_x(start.x, area).unwrap_or(0)
                } else {
                    area.x
                };

                let end = if end.y == current_line {
                    // if this is the last line in the selection
                    self.get_relative_x(end.x, area)
                } else {
                    Some(max_x)
                };

                if start < area.right()
                    && let Some(end) = end
                    && end >= area.x
                {
                    let draw_from = start.max(area.x);
                    let draw_to = end.min(area.right());

                    for x in draw_from..=draw_to {
                        buf[(x, y)].bg = ratatui::style::Color::Gray;
                    }
                }
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
