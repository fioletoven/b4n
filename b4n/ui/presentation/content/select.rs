use b4n_config::keys::KeyCombination;
use b4n_tui::{MouseEvent, MouseEventKind, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{layout::Rect, widgets::Widget};

use crate::ui::presentation::{Content, content::search::PagePosition};

/// Context for the selected text.
#[derive(Default)]
pub struct SelectContext {
    pub start: Option<PagePosition>,
    pub end: Option<PagePosition>,
}

impl SelectContext {
    /// Clears the current selection.
    pub fn clear_selection(&mut self) {
        self.start = None;
        self.end = None;
    }

    /// Clears the current selection start (if end is not set).
    pub fn clear_selection_if_partial(&mut self) {
        if self.end.is_none() {
            self.start = None;
        }
    }

    /// Returns ordered selection range if anything is selected.
    pub fn get_selection(&self) -> Option<(PagePosition, PagePosition)> {
        let (Some(start), Some(end)) = (self.start, self.end) else {
            return None;
        };
        Some(sort(start, end))
    }

    /// Returns the selection end position along with a flag indicating whether
    /// the end position comes after the start position (`true`) or not (`false`).
    pub fn get_selection_end(&self) -> Option<(PagePosition, bool)> {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            Some((end, is_sorted(start, end)))
        } else {
            None
        }
    }

    /// Process UI key/mouse event.
    pub fn process_event<T: Content>(
        &mut self,
        event: &TuiEvent,
        content: &mut T,
        page_start: &mut PagePosition,
        cursor: Option<PagePosition>,
        area: Rect,
    ) {
        match event {
            TuiEvent::Key(key) => self.process_key_event(key, cursor),
            TuiEvent::Mouse(mouse) => self.process_mouse_event(mouse, content, page_start, area),
        }
    }

    /// Updates selection end to the current cursor position only for appropriate key combinations.\
    /// **Note** that it must be executed only in edit mode and after processing edit events.
    pub fn process_event_final<T: Content>(&mut self, event: &TuiEvent, content: &T, cursor: PagePosition) {
        let TuiEvent::Key(key) = event else {
            return;
        };

        if key.modifiers != KeyModifiers::SHIFT {
            return;
        }

        if let Some(start) = self.start
            && is_allowed_key_code(key.code)
        {
            if is_sorted(start, cursor) {
                self.end = Some(decrement_curosr_x(cursor, content))
            } else {
                self.end = Some(cursor);
            }
        }
    }

    fn process_key_event(&mut self, key: &KeyCombination, cursor: Option<PagePosition>) {
        let Some(cursor) = cursor else {
            // if we are not in the edit mode just return
            return;
        };

        if key.modifiers != KeyModifiers::SHIFT {
            self.clear_selection();
            return;
        }

        if is_allowed_key_code(key.code) {
            if self.start.is_none() {
                self.start = Some(cursor);
            }
        } else {
            self.clear_selection();
        }
    }

    fn process_mouse_event<T: Content>(
        &mut self,
        mouse: &MouseEvent,
        content: &mut T,
        page_start: &mut PagePosition,
        area: Rect,
    ) {
        if mouse.kind == MouseEventKind::LeftDoubleClick {
            if let Some(pos) = get_position_in_content(area, content, *page_start, None, mouse.column, mouse.row)
                && let Some((start, end)) = content.word_bounds(pos.y, pos.x)
            {
                self.start = Some(PagePosition { x: start, y: pos.y });
                self.end = Some(PagePosition { x: end, y: pos.y });
            }
        } else if mouse.kind == MouseEventKind::LeftClick {
            self.start = get_position_in_content(area, content, *page_start, None, mouse.column, mouse.row);
            self.end = None;
        } else if mouse.kind == MouseEventKind::LeftDrag {
            scroll_page_if_needed(area, page_start, content, mouse.column, mouse.row);
            self.end = get_position_in_content(area, content, *page_start, self.start, mouse.column, mouse.row);
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
    selection_start: Option<PagePosition>,
    screen_x: u16,
    screen_y: u16,
) -> Option<PagePosition> {
    let x = page_start.x.saturating_add(screen_x.saturating_sub(area.x).into());
    let y = page_start.y.saturating_add(screen_y.saturating_sub(area.y).into());

    if y >= content.len() {
        let y = content.len().saturating_sub(1);
        let x = content.line_size(y).saturating_sub(1);
        return Some(PagePosition { x, y });
    }

    let line_len = content.line_size(y);
    if let Some(start) = selection_start {
        // we already have a selection start
        if start.y == y && start.x >= line_len && x >= line_len {
            // selection started on the same line and outside the text, return nothing
            None
        } else if is_sorted(PagePosition { x, y }, start) {
            // selection end is before selection start
            Some(PagePosition { x: x.min(line_len), y })
        } else {
            let x = x.min(line_len);
            Some(decrement_curosr_x(PagePosition { x, y }, content))
        }
    } else {
        // this is the start of a selection
        Some(PagePosition { x: x.min(line_len), y })
    }
}

fn decrement_curosr_x<T: Content>(cursor: PagePosition, content: &T) -> PagePosition {
    if cursor.x > 0 {
        PagePosition {
            x: cursor.x - 1,
            y: cursor.y,
        }
    } else if cursor.y > 0 {
        PagePosition {
            x: content.line_size(cursor.y - 1),
            y: cursor.y - 1,
        }
    } else {
        cursor
    }
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
        let Some((start, end)) = self.context.get_selection() else {
            return;
        };

        for current_line in start.y..=end.y {
            if let Some(y) = self.get_relative_y(current_line, area)
                && y >= area.y
                && y < area.bottom()
                && let Some(max_x) = self.get_relative_max_len(area, current_line)
            {
                let start_x = if start.y == current_line {
                    // if this is the first line in the selection
                    self.get_relative_x(start.x, area).unwrap_or(0)
                } else {
                    area.x
                };

                let end_x = if end.y == current_line {
                    // if this is the last line in the selection
                    self.get_relative_x(end.x, area).map(|x| x.min(max_x))
                } else {
                    Some(max_x)
                };

                if start_x < area.right()
                    && let Some(end) = end_x
                    && end >= area.x
                {
                    let draw_from = start_x.max(area.x);
                    let draw_to = end.min(area.right());

                    for x in draw_from..=draw_to {
                        buf[(x, y)].bg = ratatui::style::Color::Gray;
                    }
                }
            }
        }
    }
}

fn is_sorted(p1: PagePosition, p2: PagePosition) -> bool {
    p2.y > p1.y || (p2.y == p1.y && p2.x >= p1.x)
}

fn sort(p1: PagePosition, p2: PagePosition) -> (PagePosition, PagePosition) {
    if is_sorted(p1, p2) { (p1, p2) } else { (p2, p1) }
}

fn is_allowed_key_code(key_code: KeyCode) -> bool {
    matches!(
        key_code,
        KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
    )
}
