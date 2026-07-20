use b4n_config::themes::ControlColors;
use ratatui_core::layout::{Margin, Position, Rect};
use ratatui_core::terminal::Frame;
use ratatui_core::text::Line;
use ratatui_widgets::paragraph::Paragraph;

use crate::ResponseEvent;

/// UI `CheckBox`.
pub struct CheckBox {
    pub id: usize,
    pub is_checked: bool,
    caption: &'static str,
    is_hovered: bool,
    is_focused: bool,
    colors: ControlColors,
    area: Rect,
    width: u16,
}

impl CheckBox {
    /// Creates new [`CheckBox`] instance.
    pub fn new(id: usize, caption: &'static str, is_checked: bool, colors: ControlColors) -> Self {
        Self {
            id,
            is_checked,
            caption,
            is_hovered: false,
            is_focused: false,
            colors,
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

    /// Sets whether checkbox is hovered.
    pub fn set_hover(&mut self, is_active: bool) {
        self.is_hovered = is_active;
    }

    /// Process checkbox click.
    pub fn click(&mut self) -> ResponseEvent {
        self.is_checked = !self.is_checked;
        ResponseEvent::Changed
    }

    /// Draws [`CheckBox`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let area = area.inner(Margin::new(5, 0));
        let text = format!(" {} {} ", if self.is_checked { '󰄵' } else { '' }, self.caption);
        let line = Line::styled(text, self.colors.get(self.is_hovered, self.is_focused));
        frame.render_widget(Paragraph::new(line), area);
        self.area = area;
        self.area.width = self.width;
    }
}
