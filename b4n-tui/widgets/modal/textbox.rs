use b4n_config::themes::{ControlColors, TextColors};
use ratatui_core::layout::{Margin, Position, Rect};
use ratatui_core::terminal::Frame;
use ratatui_core::text::Line;
use ratatui_widgets::paragraph::Paragraph;

use crate::ResponseEvent;
use crate::widgets::Input;

/// UI `TextBox`.
pub struct TextBox {
    pub id: usize,
    caption: &'static str,
    input: Input,
    is_focused: bool,
    normal: TextColors,
    focused: TextColors,
    area: Rect,
    width: u16,
}

impl TextBox {
    /// Creates new [`TextBox`] instance.
    pub fn new(id: usize, caption: &'static str, input_colors: TextColors, colors: &ControlColors) -> Self {
        Self {
            id,
            caption,
            input: Input::new(input_colors),
            is_focused: false,
            normal: colors.normal,
            focused: colors.focused,
            area: Rect::default(),
            width: u16::try_from(caption.chars().count()).unwrap_or_default() + 4,
        }
    }

    /// Returns `true` if provided `x` and `y` are inside the checkbox.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.area.contains(Position::new(x, y))
    }

    /// Activates or deactivates checkbox.
    pub fn set_focus(&mut self, is_active: bool) {
        self.is_focused = is_active;
    }

    /// Process checkbox click.
    pub fn click(&mut self) -> ResponseEvent {
        // TODO
        ResponseEvent::Handled
    }

    /// Draws [`TextBox`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let area = area.inner(Margin::new(5, 0));
        let colors = if self.is_focused { self.focused } else { self.normal };
        let line = Line::styled(self.caption, &colors);
        frame.render_widget(Paragraph::new(line), area);
        self.area = area;
        self.area.width = self.width;
    }
}
