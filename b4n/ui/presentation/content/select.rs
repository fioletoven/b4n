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
    pub page_area: &'a Rect,
}

impl<'a, T: Content> ContentSelectWidget<'a, T> {
    /// Creates new [`ContentSelectWidget`] instance.
    pub fn new(context: &'a SelectContext, content: &'a T, page_start: &'a PagePosition, page_area: &'a Rect) -> Self {
        Self {
            context,
            content,
            page_start,
            page_area,
        }
    }

    fn get_relative_x(&self, x: usize, area: Rect) -> u16 {
        let x = if x >= self.page_start.x {
            u16::try_from(x - self.page_start.x).unwrap_or(self.page_area.width)
        } else {
            self.page_area.width
        };

        x.saturating_add(area.x)
    }

    fn get_relative_y(&self, y: usize, area: Rect) -> u16 {
        let y = if y >= self.page_start.y {
            u16::try_from(y - self.page_start.y).unwrap_or(self.page_area.height)
        } else {
            self.page_area.height
        };

        y.saturating_add(area.y)
    }

    fn get_max_line_len(&self, area: Rect, current_line: usize) -> u16 {
        let line_len = self.content.line_size(current_line) + 1;
        u16::try_from(line_len.saturating_sub(usize::from(area.x)) + 1).unwrap_or(u16::MAX)
    }
}

impl<'a, T: Content> Widget for ContentSelectWidget<'a, T> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if let Some(start) = self.context.start
            && let Some(end) = self.context.end
        {
            let (start, end) = sort(start, end);
            for current_line in start.y..=end.y {
                let y = self.get_relative_y(current_line, area);
                if y >= area.y && y < area.bottom() {
                    let max_x = self.get_max_line_len(area, current_line);

                    let start = if start.y == current_line {
                        self.get_relative_x(start.x, area)
                    } else {
                        area.x
                    };

                    let end = if end.y == current_line {
                        self.get_relative_x(end.x, area).min(max_x)
                    } else {
                        max_x
                    };

                    for x in start..=end {
                        if x < area.right() {
                            buf[(x, y)].bg = ratatui::style::Color::Gray;
                        }
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
