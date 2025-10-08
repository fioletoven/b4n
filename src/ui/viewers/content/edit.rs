use crossterm::event::KeyCode;
use ratatui::{
    layout::{Position, Rect},
    style::Color,
    widgets::Widget,
};

use crate::ui::{MouseEventKind, ResponseEvent, TuiEvent};

use super::{Content, search::PagePosition};

#[derive(Default)]
pub struct EditContext {
    pub is_enabled: bool,
    pub cursor: PagePosition,
    last_set_x: usize,
}

impl EditContext {
    /// Sets [`EditContext`] as enabled.
    pub fn enable<T: Content>(&mut self, position: PagePosition, page_size: u16, content: &mut T) {
        self.is_enabled = true;
        if self.cursor.y < position.y {
            self.cursor.y = position.y;
            self.update_cursor_position(false, content);
        } else if self.cursor.y >= position.y + usize::from(page_size) {
            self.cursor.y = position.y + usize::from(page_size.saturating_sub(1));
            self.update_cursor_position(false, content);
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
        let mut line_size = content.line_size(self.cursor.y);

        let mut x_changed = None;
        let mut y_changed = None;

        match event {
            TuiEvent::Key(key) => match key.code {
                KeyCode::Char(c) => {
                    content.insert_char(self.cursor.x, self.cursor.y, c);
                    x_changed = Some(Some(self.cursor.x.saturating_add(1)));
                    line_size = content.line_size(self.cursor.y);
                },
                KeyCode::Backspace => {
                    if let Some((x, y)) = content.remove_char(self.cursor.x, self.cursor.y, true) {
                        x_changed = Some(Some(x));
                        y_changed = Some(y);
                        line_size = content.line_size(y);
                    }
                },
                KeyCode::Delete => {
                    if let Some((x, y)) = content.remove_char(self.cursor.x, self.cursor.y, false) {
                        x_changed = Some(Some(x));
                        y_changed = Some(y);
                        line_size = content.line_size(y);
                    }
                },
                KeyCode::Enter => {
                    content.insert_char(self.cursor.x, self.cursor.y, '\n');
                    x_changed = Some(Some(0));
                    y_changed = Some(self.cursor.y.saturating_add(1));
                    line_size = content.line_size(self.cursor.y.saturating_add(1));
                },
                _ => match key {
                    a if a.code == KeyCode::Home => x_changed = Some(Some(0)),
                    a if a.code == KeyCode::Left => x_changed = Some(self.cursor.x.checked_sub(1)),
                    a if a.code == KeyCode::Right => x_changed = Some(Some(self.cursor.x.saturating_add(1))),
                    a if a.code == KeyCode::End => x_changed = Some(Some(line_size)),

                    a if a.code == KeyCode::PageUp => y_changed = Some(self.cursor.y.saturating_sub(area.height.into())),
                    a if a.code == KeyCode::Up => y_changed = Some(self.cursor.y.saturating_sub(1)),
                    a if a.code == KeyCode::Down => y_changed = Some(self.cursor.y.saturating_add(1)),
                    a if a.code == KeyCode::PageDown => y_changed = Some(self.cursor.y.saturating_add(area.height.into())),

                    _ => return ResponseEvent::NotHandled,
                },
            },
            TuiEvent::Mouse(mouse) => match mouse {
                a if a.kind == MouseEventKind::LeftClick => {
                    self.cursor.x = position.x.saturating_add(a.column.saturating_sub(area.x).into());
                    self.cursor.y = position.y.saturating_add(a.row.saturating_sub(area.y).into());
                },

                _ => return ResponseEvent::NotHandled,
            },
        }

        if let Some(new_x) = x_changed {
            if let Some(x) = new_x {
                if x > line_size && self.cursor.y.saturating_add(1) < content.len() {
                    self.cursor.x = 0;
                    self.cursor.y = self.cursor.y.saturating_add(1);
                } else {
                    self.cursor.x = x;
                }
            } else if let Some(y) = self.cursor.y.checked_sub(1) {
                self.cursor.y = y;
                self.cursor.x = content.line_size(y);
            }

            self.last_set_x = self.cursor.x;
        }

        if let Some(new_y) = y_changed {
            self.cursor.y = new_y;
        }

        self.update_cursor_position(y_changed.is_some(), content);
        ResponseEvent::Handled
    }

    fn update_cursor_position<T: Content>(&mut self, use_last_x: bool, content: &mut T) {
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

impl<'a> Widget for ContentEditWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
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
                cell.bg = Color::Gray;
            }
        }
    }
}
