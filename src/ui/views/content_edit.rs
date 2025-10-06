use crossterm::event::KeyCode;
use ratatui::{
    layout::{Position, Rect},
    style::Color,
    widgets::Widget,
};

use crate::ui::{
    MouseEventKind, ResponseEvent, TuiEvent,
    views::{content::Content, content_search::PagePosition},
};

pub struct EditContext {
    pub cursor: PagePosition,
    last_set_x: usize,
}

impl EditContext {
    pub fn new(position: PagePosition) -> Self {
        Self {
            cursor: position,
            last_set_x: 0,
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

        let lines_no = content.len();
        if let Some(new_x) = x_changed {
            if let Some(x) = new_x {
                if x > line_size && self.cursor.y.saturating_add(1) < lines_no {
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

        if self.cursor.y >= lines_no {
            self.cursor.y = lines_no.saturating_sub(1);
        }

        let line_size = content.line_size(self.cursor.y);
        if self.cursor.x > line_size {
            self.cursor.x = line_size;
        } else if y_changed.is_some() && self.cursor.x < self.last_set_x {
            self.cursor.x = self.last_set_x.min(line_size);
        }

        ResponseEvent::Handled
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
