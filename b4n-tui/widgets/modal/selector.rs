use b4n_config::themes::{ControlColors, SelectColors, TextColors};
use ratatui::layout::{Margin, Position, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::ResponseEvent;
use crate::widgets::{ActionsList, Select};

/// UI `Selector`.
pub struct Selector {
    pub id: usize,
    options: Select<ActionsList>,
    is_focused: bool,
    caption: &'static str,
    normal: TextColors,
    focused: TextColors,
    area: Rect,
    width: u16,
}

impl Selector {
    /// Creates new [`Selector`] instance.
    pub fn new(id: usize, caption: &'static str, options: ActionsList, colors: &ControlColors) -> Self {
        let mut options = Select::new(options, SelectColors::default(), false, false);
        options.disable_filter(true);

        Self {
            id,
            options,
            is_focused: false,
            caption,
            normal: colors.normal,
            focused: colors.focused,
            area: Rect::default(),
            width: u16::try_from(caption.chars().count()).unwrap_or_default() + 4,
        }
    }

    /// Returns `true` if provided `x` and `y` are inside the selector.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.area.contains(Position::new(x, y))
    }

    /// Activates or deactivates selector.
    pub fn set_focus(&mut self, is_active: bool) {
        self.is_focused = is_active;
    }

    /// Process selector click.
    pub fn click(&mut self) -> ResponseEvent {
        ResponseEvent::Handled
    }

    /// Draws [`Selector`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let area = area.inner(Margin::new(5, 0));
        let colors = if self.is_focused { self.focused } else { self.normal };
        let text = format!("   {} ", &self.caption);
        let line = Line::styled(text, &colors);
        frame.render_widget(Paragraph::new(line), area);
        self.area = area;
        self.area.width = self.width;
    }
}
