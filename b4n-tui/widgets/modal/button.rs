use b4n_config::themes::{ControlColors, TextColors};
use ratatui_core::layout::{Position, Rect};
use ratatui_core::terminal::Frame;
use ratatui_core::text::Line;
use ratatui_widgets::paragraph::Paragraph;

use crate::ResponseEvent;

/// UI `Button`.
pub struct Button {
    is_focused: bool,
    caption: &'static str,
    normal: TextColors,
    focused: TextColors,
    result: ResponseEvent,
    area: Rect,
}

impl Button {
    /// Creates new [`Button`] instance.
    pub fn new(caption: &'static str, result: ResponseEvent, colors: &ControlColors) -> Self {
        Self {
            is_focused: false,
            caption,
            normal: colors.normal,
            focused: colors.focused,
            result,
            area: Rect::default(),
        }
    }

    /// Returns length of the caption.
    pub fn len(&self) -> u16 {
        (self.caption.chars().count() + 3) as u16
    }

    /// Returns `true` if this button has no caption, and false otherwise.
    pub fn is_empty(&self) -> bool {
        self.caption.is_empty()
    }

    /// Returns `true` if provided `x` and `y` are inside the button.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.area.contains(Position::new(x, y))
    }

    /// Returns button result.
    pub fn result(&self) -> ResponseEvent {
        self.result.clone()
    }

    /// Activates or deactivates button.
    pub fn set_focus(&mut self, is_active: bool) {
        self.is_focused = is_active;
    }

    /// Draws [`Button`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let colors = if self.is_focused { self.focused } else { self.normal };
        let line = Line::styled(format!(" {} ", &self.caption), &colors);
        frame.render_widget(Paragraph::new(line), area);
        self.area = area;
    }
}
