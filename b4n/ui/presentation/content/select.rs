use b4n_config::keys::KeyCombination;
use b4n_tui::{MouseEvent, MouseEventKind, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{layout::Rect, style::Color, widgets::Widget};

use crate::ui::presentation::{Content, content::search::ContentPosition};

/// Represents a text selection defined by a `start` and `end` position in the content.
#[derive(Clone)]
pub struct Selection {
    pub start: ContentPosition,
    pub end: ContentPosition,
}

impl Selection {
    /// Creates new [`Selection`] instance.
    pub fn new(start: ContentPosition, end: ContentPosition) -> Self {
        Self { start, end }
    }

    /// Returns `true` if `end` comes after `start` in document order.
    pub fn is_end_after_start(&self) -> bool {
        is_sorted(self.start, self.end)
    }

    /// Returns the two positions sorted in document order, ensuring that the first is always the earlier position.
    pub fn sorted(&self) -> (ContentPosition, ContentPosition) {
        sort(self.start, self.end)
    }
}

/// Context for the selected text.
#[derive(Default)]
pub struct SelectContext {
    pub start: Option<ContentPosition>,
    pub end: Option<ContentPosition>,
    init: Option<ContentPosition>,
}

impl SelectContext {
    /// Clears the current selection.
    pub fn clear_selection(&mut self) {
        self.start = None;
        self.end = None;
        self.init = None;
    }

    /// Clears the current selection start (if end is not set) or adjusts the `init` of the selection
    /// to match situation when cursor is after selection start.
    pub fn adjust_selection(&mut self) {
        if self.end.is_none() {
            self.start = None;
            self.init = None;
        } else {
            self.init = self.start;
        }
    }

    /// Returns a text selection.
    pub fn get_selection(&self) -> Option<Selection> {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            Some(Selection::new(start, end))
        } else {
            None
        }
    }

    /// Process UI key/mouse event.
    pub fn process_event<T: Content>(
        &mut self,
        event: &TuiEvent,
        content: &mut T,
        page_start: &mut ContentPosition,
        cursor: Option<ContentPosition>,
        area: Rect,
    ) {
        match event {
            TuiEvent::Key(key) => self.process_key_event(key, cursor),
            TuiEvent::Mouse(mouse) => self.process_mouse_event(mouse, content, page_start, area),
        }
    }

    /// Updates selection end to the current cursor position only for appropriate key combinations.\
    /// **Note** that it must be executed only in edit mode and after processing edit events.
    pub fn process_event_final<T: Content>(&mut self, event: &TuiEvent, content: &T, cursor: ContentPosition) {
        let TuiEvent::Key(key) = event else {
            return;
        };

        if key.modifiers != KeyModifiers::SHIFT {
            self.clear_selection();
            return;
        }

        if let Some(init) = self.init
            && is_allowed_key_code(key.code)
        {
            if is_sorted(init, cursor) {
                self.start = self.init;
                if init == cursor {
                    self.end = None;
                } else {
                    self.end = Some(decrement_cursor_x(cursor, content));
                }
            } else {
                self.start = Some(decrement_cursor_x(init, content));
                self.end = Some(cursor);
            }
        }
    }

    fn process_key_event(&mut self, key: &KeyCombination, cursor: Option<ContentPosition>) {
        let Some(cursor) = cursor else {
            // if we are not in the edit mode just return
            return;
        };

        if key.modifiers != KeyModifiers::SHIFT {
            return;
        }

        if is_allowed_key_code(key.code) {
            if self.init.is_none() {
                self.init = Some(cursor);
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
        page_start: &mut ContentPosition,
        area: Rect,
    ) {
        if mouse.kind == MouseEventKind::LeftDoubleClick {
            if let Some(pos) = get_position_in_content(area, content, *page_start, None, mouse.column, mouse.row)
                && let Some((start, end)) = content.word_bounds(pos.y, pos.x)
            {
                self.init = Some(ContentPosition { x: start, y: pos.y });
                self.start = self.init;
                self.end = Some(ContentPosition { x: end, y: pos.y });
            }
        } else if mouse.kind == MouseEventKind::LeftClick {
            self.init = get_position_in_content(area, content, *page_start, None, mouse.column, mouse.row);
            self.start = self.init;
            self.end = None;
        } else if mouse.kind == MouseEventKind::LeftDrag {
            scroll_page_if_needed(area, page_start, content, mouse.column, mouse.row);
            self.end = get_position_in_content(area, content, *page_start, self.init, mouse.column, mouse.row);
            if let Some(init) = self.init
                && let Some(end) = self.end
            {
                if is_sorted(init, end) {
                    self.start = self.init;
                } else {
                    self.start = Some(decrement_cursor_x(init, content));
                }
            }
        }
    }
}

fn scroll_page_if_needed<T: Content>(area: Rect, page_start: &mut ContentPosition, content: &mut T, mouse_x: u16, mouse_y: u16) {
    // scroll page vertically while dragging
    if mouse_y > (area.y + area.height).saturating_sub(3) {
        page_start.y += 2;
    } else if mouse_y < area.y + 3 {
        page_start.y = page_start.y.saturating_sub(2);
    }

    // scroll page horizontally while dragging
    if mouse_x > (area.x + area.width).saturating_sub(3) {
        page_start.x += 2;
    } else if mouse_x < area.x + 3 {
        page_start.x = page_start.x.saturating_sub(2);
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
    page_start: ContentPosition,
    selection_start: Option<ContentPosition>,
    screen_x: u16,
    screen_y: u16,
) -> Option<ContentPosition> {
    let x = page_start.x.saturating_add(screen_x.saturating_sub(area.x).into());
    let y = page_start.y.saturating_add(screen_y.saturating_sub(area.y).into());

    if y >= content.len() {
        let y = content.len().saturating_sub(1);
        let x = content.line_size(y).saturating_sub(1);
        return Some(ContentPosition { x, y });
    }

    let line_len = content.line_size(y);
    if let Some(start) = selection_start {
        // we already have a selection start
        if start.y == y && start.x >= line_len && x >= line_len {
            // selection started on the same line and outside the text, return nothing
            None
        } else if is_sorted(ContentPosition { x, y }, start) {
            // selection end is before selection start
            Some(ContentPosition { x: x.min(line_len), y })
        } else {
            let x = x.min(line_len);
            Some(decrement_cursor_x(ContentPosition { x, y }, content))
        }
    } else {
        // this is the start of a selection
        Some(ContentPosition { x: x.min(line_len), y })
    }
}

fn decrement_cursor_x<T: Content>(cursor: ContentPosition, content: &T) -> ContentPosition {
    if cursor.x > 0 {
        ContentPosition {
            x: cursor.x - 1,
            y: cursor.y,
        }
    } else if cursor.y > 0 {
        ContentPosition {
            x: content.line_size(cursor.y - 1),
            y: cursor.y - 1,
        }
    } else {
        cursor
    }
}

/// Widget that draws selection on the content.
pub struct ContentSelectWidget<'a, T: Content> {
    context: &'a SelectContext,
    content: &'a T,
    page_start: &'a ContentPosition,
    color: Color,
}

impl<'a, T: Content> ContentSelectWidget<'a, T> {
    /// Creates new [`ContentSelectWidget`] instance.
    pub fn new(context: &'a SelectContext, content: &'a T, page_start: &'a ContentPosition, color: Color) -> Self {
        Self {
            context,
            content,
            page_start,
            color,
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

impl<T: Content> Widget for ContentSelectWidget<'_, T> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let Some((start, end)) = self.context.get_selection().map(|s| s.sorted()) else {
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
                        buf[(x, y)].bg = self.color;
                    }
                }
            }
        }
    }
}

fn is_sorted(p1: ContentPosition, p2: ContentPosition) -> bool {
    p2.y > p1.y || (p2.y == p1.y && p2.x >= p1.x)
}

fn sort(p1: ContentPosition, p2: ContentPosition) -> (ContentPosition, ContentPosition) {
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
