use b4n_config::{keys::KeyCombination, themes::TextColors};
use b4n_tui::{MouseEvent, MouseEventKind, ResponseEvent, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::widgets::Widget;
use std::time::Instant;

use crate::ui::presentation::Selection;
use crate::ui::presentation::content::{Content, search::ContentPosition};

/// Context for the content edit mode.
pub struct EditContext {
    pub is_enabled: bool,
    pub is_modified: bool,
    pub cursor: ContentPosition,
    color: TextColors,
    last_set_x: usize,
    last_key_press: Instant,
}

impl EditContext {
    /// Creates new [`EditContext`] instance.
    pub fn new(color: TextColors) -> Self {
        Self {
            is_enabled: false,
            is_modified: false,
            cursor: ContentPosition::default(),
            color,
            last_set_x: 0,
            last_key_press: Instant::now(),
        }
    }

    /// Sets [`EditContext`] as enabled.
    pub fn enable<T: Content>(
        &mut self,
        page_start: ContentPosition,
        selection: Option<Selection>,
        cursor_start: Option<ContentPosition>,
        page_size: u16,
        content: &mut T,
    ) {
        self.is_enabled = true;
        if let Some(selection) = selection {
            self.cursor = get_cursor_pos_for_selection(content, selection.end, selection.is_end_after_start());
            self.constraint_cursor_position(false, content);
        } else if let Some(cursor_start) = cursor_start {
            self.cursor = cursor_start;
            self.constraint_cursor_position(false, content);
        } else if self.cursor.y < page_start.y {
            self.cursor.y = page_start.y;
            self.constraint_cursor_position(false, content);
        } else if self.cursor.y >= page_start.y + usize::from(page_size) {
            self.cursor.y = page_start.y + usize::from(page_size.saturating_sub(1));
            self.constraint_cursor_position(false, content);
        }
        self.last_set_x = self.cursor.x;
    }

    /// Process UI key/mouse event.
    pub fn process_event<T: Content>(
        &mut self,
        event: &TuiEvent,
        content: &mut T,
        page_start: ContentPosition,
        selection: Option<Selection>,
        area: Rect,
    ) -> ResponseEvent {
        if event.is_key(&KeyCombination::new(KeyCode::Char('a'), KeyModifiers::CONTROL)) {
            let last = content.len().saturating_sub(1);
            self.cursor = ContentPosition::new(content.line_size(last), last);
            self.last_key_press = Instant::now();
            return ResponseEvent::Handled;
        }

        match event {
            TuiEvent::Key(key) => {
                let pos = if key == &KeyCombination::new(KeyCode::Char('z'), KeyModifiers::CONTROL) {
                    content.undo().map_or((None, None), |pos| (Some(Some(pos.x)), Some(pos.y)))
                } else if key == &KeyCombination::new(KeyCode::Char('y'), KeyModifiers::CONTROL) {
                    content.redo().map_or((None, None), |pos| (Some(Some(pos.x)), Some(pos.y)))
                } else {
                    self.process_key(key, content, selection, area)
                };
                self.update_cursor_position(pos, content, false);
                self.last_key_press = Instant::now();
            },
            TuiEvent::Mouse(mouse) => {
                if mouse.kind == MouseEventKind::LeftClick {
                    let pos = self.process_mouse(*mouse, page_start, area);
                    self.update_cursor_position(pos, content, true);
                } else {
                    if let Some(selection) = selection {
                        self.cursor = get_cursor_pos_for_selection(content, selection.end, selection.is_end_after_start());
                    }

                    return ResponseEvent::NotHandled;
                }
            },
            TuiEvent::Command(_) => (),
        }

        ResponseEvent::Handled
    }

    fn process_key<T: Content>(
        &mut self,
        key: &KeyCombination,
        content: &mut T,
        selection: Option<Selection>,
        area: Rect,
    ) -> NewCursorPosition {
        if selection.is_none() {
            if key == &KeyCombination::new(KeyCode::Up, KeyModifiers::ALT) && self.cursor.y > 0 {
                content.swap_lines(self.cursor.y.saturating_sub(1), self.cursor.y);
            }

            if key == &KeyCombination::new(KeyCode::Down, KeyModifiers::ALT) && self.cursor.y + 1 < content.len() {
                content.swap_lines(self.cursor.y, self.cursor.y + 1);
            }
        }

        let is_ctrl_x = key == &KeyCombination::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        if (is_hiding_selection_key(key) || is_ctrl_x)
            && let Some(selection) = selection
        {
            let start = selection.sorted().0;
            content.remove_text(selection);

            if key.code == KeyCode::Backspace || key.code == KeyCode::Delete || is_ctrl_x {
                return (Some(Some(start.x)), Some(start.y));
            }

            self.cursor = start;
        }

        let is_ctrl_d = key == &KeyCombination::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        if is_ctrl_x || is_ctrl_d {
            let range = Selection::from_line_end(content.line_size(self.cursor.y), self.cursor.y);
            content.remove_text(range);
            return (None, None);
        }

        self.process_key_code(key.code, content, area)
    }

    fn process_key_code<T: Content>(&mut self, key: KeyCode, content: &mut T, area: Rect) -> NewCursorPosition {
        let mut x_changed = None;
        let mut y_changed = None;

        match key {
            // insert character
            KeyCode::Char(c) => {
                content.insert_char(self.cursor, c);
                x_changed = Some(Some(self.cursor.x + 1));
            },
            KeyCode::Tab => {
                content.insert_char(self.cursor, ' ');
                content.insert_char(self.cursor, ' ');
                x_changed = Some(Some(self.cursor.x + 2));
            },
            KeyCode::Enter => {
                content.insert_char(self.cursor, '\n');
                y_changed = Some(self.cursor.y + 1);
                if self.last_key_press.elapsed().as_millis() > 10
                    && let Some(leading_spaces) = content.leading_spaces(self.cursor.y)
                {
                    for i in 0..leading_spaces {
                        content.insert_char(ContentPosition::new(i, self.cursor.y + 1), ' ');
                    }
                    x_changed = Some(Some(leading_spaces));
                } else {
                    x_changed = Some(Some(0));
                }
            },

            // remove character
            KeyCode::Backspace => {
                if let Some(position) = content.remove_char(self.cursor, true) {
                    x_changed = Some(Some(position.x));
                    y_changed = Some(position.y);
                }
            },
            KeyCode::Delete => {
                if let Some(position) = content.remove_char(self.cursor, false) {
                    x_changed = Some(Some(position.x));
                    y_changed = Some(position.y);
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

    fn process_mouse(&mut self, mouse: MouseEvent, page_start: ContentPosition, area: Rect) -> NewCursorPosition {
        if mouse.kind == MouseEventKind::LeftClick {
            let x = page_start.x.saturating_add(mouse.column.saturating_sub(area.x).into());
            let y = page_start.y.saturating_add(mouse.row.saturating_sub(area.y).into());
            let x = if self.cursor.x == x { None } else { Some(Some(x)) };
            let y = if self.cursor.y == y { None } else { Some(y) };
            return (x, y);
        }

        (None, None)
    }

    fn update_cursor_position<T: Content>(&mut self, mut pos: NewCursorPosition, content: &mut T, is_mouse: bool) {
        if let Some(new_x) = pos.0 {
            if let Some(x) = new_x {
                let pos_y = pos.1.unwrap_or(self.cursor.y);
                let line_size = content.line_size(pos_y);
                if !is_mouse && x > line_size && pos_y.saturating_add(1) < content.len() {
                    self.cursor.x = 0;
                    self.cursor.y = pos_y.saturating_add(1);
                    pos.1 = None; // if we changed cursor.y here, we do not want to overwrite it later
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

fn is_hiding_selection_key(key: &KeyCombination) -> bool {
    matches!(
        key.code,
        KeyCode::Char(_) | KeyCode::Tab | KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete
    )
}

fn get_cursor_pos_for_selection<T: Content>(content: &T, end: ContentPosition, end_after_start: bool) -> ContentPosition {
    if !end_after_start {
        return end;
    }

    let line_len = content.line_size(end.y);
    if end.x < line_len {
        return ContentPosition { x: end.x + 1, y: end.y };
    }

    if end.y + 1 >= content.len() {
        ContentPosition { x: line_len, y: end.y }
    } else {
        ContentPosition { x: 0, y: end.y + 1 }
    }
}

type NewCursorPosition = (Option<Option<usize>>, Option<usize>);

/// Widget that draws cursor on the content.
pub struct ContentEditWidget<'a> {
    pub context: &'a EditContext,
    pub page_start: &'a ContentPosition,
}

impl<'a> ContentEditWidget<'a> {
    /// Creates new [`ContentEditWidget`] instance.
    pub fn new(context: &'a EditContext, page_start: &'a ContentPosition) -> Self {
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
                cell.bg = self.context.color.bg;
                if self.context.color.fg != Color::Reset {
                    cell.fg = self.context.color.fg;
                }
            }
        }
    }
}
