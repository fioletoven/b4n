use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Position, Rect},
    widgets::Widget,
};

use crate::ui::{KeyCombination, MouseEvent, MouseEventKind, ResponseEvent, TuiEvent, colors::TextColors};

use super::{Content, search::PagePosition};

#[derive(Default)]
pub struct EditContext {
    pub is_enabled: bool,
    pub is_modified: bool,
    pub cursor: PagePosition,
    color: TextColors,
    last_set_x: usize,
}

impl EditContext {
    /// Creates new [`EditContext`] instance.
    pub fn new(color: TextColors) -> Self {
        Self {
            color,
            ..Default::default()
        }
    }

    /// Sets [`EditContext`] as enabled.
    pub fn enable<T: Content>(&mut self, position: PagePosition, page_size: u16, content: &mut T) {
        self.is_enabled = true;
        if self.cursor.y < position.y {
            self.cursor.y = position.y;
            self.constraint_cursor_position(false, content);
        } else if self.cursor.y >= position.y + usize::from(page_size) {
            self.cursor.y = position.y + usize::from(page_size.saturating_sub(1));
            self.constraint_cursor_position(false, content);
        }
    }

    /// Process UI key/mouse event.
    pub fn process_event<T: Content>(
        &mut self,
        event: &TuiEvent,
        content: &mut T,
        position: PagePosition,
        area: Rect,
    ) -> ResponseEvent {
        match event {
            TuiEvent::Key(key) => {
                let pos = if key == &KeyCombination::new(KeyCode::Char('z'), KeyModifiers::CONTROL) {
                    content.undo().map_or((None, None), |(x, y)| (Some(Some(x)), Some(y)))
                } else if key == &KeyCombination::new(KeyCode::Char('y'), KeyModifiers::CONTROL) {
                    content.redo().map_or((None, None), |(x, y)| (Some(Some(x)), Some(y)))
                } else {
                    self.process_key(key.code, content, area)
                };
                self.update_cursor_position(pos, content, false);
            },
            TuiEvent::Mouse(mouse) => {
                if mouse.kind == MouseEventKind::LeftClick {
                    let pos = self.process_mouse(*mouse, position, area);
                    self.update_cursor_position(pos, content, true);
                } else {
                    return ResponseEvent::NotHandled;
                }
            },
        }

        ResponseEvent::Handled
    }

    fn process_key<T: Content>(&mut self, key: KeyCode, content: &mut T, area: Rect) -> NewCursorPosition {
        let mut x_changed = None;
        let mut y_changed = None;

        match key {
            // insert character
            KeyCode::Char(c) => {
                content.insert_char(self.cursor.x, self.cursor.y, c);
                x_changed = Some(Some(self.cursor.x + 1));
            },
            KeyCode::Tab => {
                content.insert_char(self.cursor.x, self.cursor.y, ' ');
                content.insert_char(self.cursor.x, self.cursor.y, ' ');
                x_changed = Some(Some(self.cursor.x + 2));
            },
            KeyCode::Enter => {
                content.insert_char(self.cursor.x, self.cursor.y, '\n');
                y_changed = Some(self.cursor.y + 1);
                if let Some(leading_spaces) = content.leading_spaces(self.cursor.y) {
                    for i in 0..leading_spaces {
                        content.insert_char(i, self.cursor.y + 1, ' ');
                    }
                    x_changed = Some(Some(leading_spaces));
                } else {
                    x_changed = Some(Some(0));
                }
            },

            // remove character
            KeyCode::Backspace => {
                if let Some((x, y)) = content.remove_char(self.cursor.x, self.cursor.y, true) {
                    x_changed = Some(Some(x));
                    y_changed = Some(y);
                }
            },
            KeyCode::Delete => {
                if let Some((x, y)) = content.remove_char(self.cursor.x, self.cursor.y, false) {
                    x_changed = Some(Some(x));
                    y_changed = Some(y);
                }
            },

            // navigate horizontal
            KeyCode::Home => x_changed = Some(Some(0)),
            KeyCode::Left => x_changed = Some(self.cursor.x.checked_sub(1)),
            KeyCode::Right => x_changed = Some(Some(self.cursor.x + 1)),
            KeyCode::End => x_changed = Some(Some(content.line_size(self.cursor.y))),

            // navigate vertical
            KeyCode::PageUp => y_changed = Some(self.cursor.y.saturating_sub(area.height.into())),
            KeyCode::Up => y_changed = Some(self.cursor.y.saturating_sub(1)),
            KeyCode::Down => y_changed = Some(self.cursor.y + 1),
            KeyCode::PageDown => y_changed = Some(self.cursor.y.saturating_add(area.height.into())),

            _ => (),
        }

        (x_changed, y_changed)
    }

    fn process_mouse(&mut self, mouse: MouseEvent, position: PagePosition, area: Rect) -> NewCursorPosition {
        if mouse.kind == MouseEventKind::LeftClick {
            let x = position.x.saturating_add(mouse.column.saturating_sub(area.x).into());
            let y = position.y.saturating_add(mouse.row.saturating_sub(area.y).into());
            let x = if self.cursor.x == x { None } else { Some(Some(x)) };
            let y = if self.cursor.y == y { None } else { Some(y) };
            return (x, y);
        }

        (None, None)
    }

    fn update_cursor_position<T: Content>(&mut self, pos: NewCursorPosition, content: &mut T, is_mouse: bool) {
        if let Some(new_x) = pos.0 {
            if let Some(x) = new_x {
                let line_size = content.line_size(pos.1.unwrap_or(self.cursor.y));
                if !is_mouse && x > line_size && self.cursor.y.saturating_add(1) < content.len() {
                    self.cursor.x = 0;
                    self.cursor.y = self.cursor.y.saturating_add(1);
                } else {
                    self.cursor.x = x;
                }
            } else if let Some(y) = self.cursor.y.checked_sub(1) {
                self.cursor.y = y;
                self.cursor.x = content.line_size(y);
            }
        }

        if let Some(new_y) = pos.1 {
            self.cursor.y = new_y;
        }

        // we can set `x` to the last set value only if this is move on `y` axe, so if:
        // pos.0 was not changed, and pos.1 was changed, and it is not a mouse event
        let use_last_x = pos.0.is_none() && pos.1.is_some() && !is_mouse;
        self.constraint_cursor_position(use_last_x, content);

        if pos.0.is_some() {
            self.last_set_x = self.cursor.x;
        }
    }

    fn constraint_cursor_position<T: Content>(&mut self, use_last_x: bool, content: &mut T) {
        let lines_no = content.len();
        if self.cursor.y >= lines_no {
            self.cursor.y = lines_no.saturating_sub(1);
        }

        let line_size = content.line_size(self.cursor.y);
        if self.cursor.x > line_size {
            self.cursor.x = line_size;
        } else if use_last_x && self.cursor.x < self.last_set_x {
            self.cursor.x = self.last_set_x.min(line_size);
        }
    }
}

type NewCursorPosition = (Option<Option<usize>>, Option<usize>);

/// Widget that draws cursor on the content.
pub struct ContentEditWidget<'a> {
    pub context: &'a EditContext,
    pub page_start: &'a PagePosition,
}

impl<'a> ContentEditWidget<'a> {
    /// Creates new [`ContentEditWidget`] instance.
    pub fn new(context: &'a EditContext, page_start: &'a PagePosition) -> Self {
        Self { context, page_start }
    }
}

impl Widget for ContentEditWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if let Some(x) = self.context.cursor.x.checked_sub(self.page_start.x)
            && let Some(y) = self.context.cursor.y.checked_sub(self.page_start.y)
        {
            let cursor = Position {
                x: u16::try_from(x.saturating_add(area.x.into())).unwrap_or_default(),
                y: u16::try_from(y.saturating_add(area.y.into())).unwrap_or_default(),
            };

            if area.contains(cursor)
                && let Some(cell) = buf.cell_mut(cursor)
            {
                cell.fg = self.context.color.fg;
                cell.bg = self.context.color.bg;
            }
        }
    }
}
