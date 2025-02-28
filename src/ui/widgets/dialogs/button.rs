use ratatui::{layout::Rect, text::Line, widgets::Paragraph};

use crate::ui::{ResponseEvent, colors::TextColors, theme::ButtonColors};

/// UI Button.
pub struct Button {
    is_focused: bool,
    caption: String,
    normal: TextColors,
    focused: TextColors,
    result: ResponseEvent,
}

impl Button {
    /// Creates new [`Button`] instance.
    pub fn new(caption: String, result: ResponseEvent, colors: ButtonColors) -> Self {
        Self {
            is_focused: false,
            caption,
            normal: colors.normal,
            focused: colors.focused,
            result,
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

    /// Returns button result.
    pub fn result(&self) -> ResponseEvent {
        self.result.clone()
    }

    /// Activates or deactivates button.
    pub fn set_focus(&mut self, is_active: bool) {
        self.is_focused = is_active;
    }

    /// Draws [`Button`] on the provided frame area.
    pub fn draw(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let colors = if self.is_focused { self.focused } else { self.normal };
        let line = Line::styled(format!(" {} ", &self.caption), &colors);
        frame.render_widget(Paragraph::new(line), area);
    }
}
