use b4n_config::themes::TextBoxModalColors;
use crossterm::event::KeyCode;
use ratatui_core::layout::{Margin, Position, Rect};
use ratatui_core::terminal::Frame;
use ratatui_core::text::Line;
use ratatui_widgets::paragraph::Paragraph;

use crate::widgets::Input;
use crate::{ResponseEvent, Responsive, TuiEvent};

/// UI `TextBox`.
pub struct TextBox {
    pub id: usize,
    caption: &'static str,
    input: Input,
    prev_value: String,
    is_hovered: bool,
    is_focused: bool,
    show_cursor: bool,
    colors: TextBoxModalColors,
    area: Rect,
    caption_width: u16,
    input_width: u16,
}

impl TextBox {
    /// Creates new [`TextBox`] instance.
    pub fn new(id: usize, caption: &'static str, input_width: u16, colors: TextBoxModalColors) -> Self {
        let mut input = Input::new(colors.caption.normal);
        input.set_cursor_colors(colors.cursor);

        Self {
            id,
            caption,
            input,
            prev_value: String::new(),
            is_hovered: false,
            is_focused: false,
            show_cursor: colors.cursor.is_some(),
            colors,
            area: Rect::default(),
            caption_width: u16::try_from(caption.chars().count()).unwrap_or_default() + 4,
            input_width,
        }
    }

    /// Sets initial value for the textbox.
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.input.set_value(value);
        self
    }

    /// Adds button to the textbox.
    pub fn with_button(mut self, icon: &'static str, action: &'static str) -> Self {
        let response = ResponseEvent::Action(action);
        self.input
            .set_accept_button(Some((icon, response)), Some(self.colors.caption.normal));
        self
    }

    /// Sets new caption for the textbox.
    pub fn set_caption(&mut self, caption: &'static str) {
        self.caption_width = u16::try_from(caption.chars().count()).unwrap_or_default() + 4;
        self.caption = caption;
    }

    /// Sets whether to show button.
    pub fn show_button(&mut self, is_visible: bool) {
        self.input.highlight_accept_button(false);
        self.input.show_accept_button(is_visible);
    }

    /// Returns value set in the textbox.
    pub fn value(&self) -> &str {
        self.input.value_full()
    }

    /// Returns `true` if provided `x` and `y` are inside the textbox.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.area.contains(Position::new(x, y))
    }

    /// Returns `true` if textbox is focused.
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Activates or deactivates textbox.
    pub fn set_focus(&mut self, is_active: bool) {
        self.is_focused = is_active;
        self.input.show_cursor(is_active && self.show_cursor);

        let colors = self.colors.caption.get(self.is_hovered, self.is_focused);
        self.input.set_accept_button_colors(Some(*colors));
        if is_active {
            self.input.set_colors(self.colors.input);
        } else {
            self.input.set_colors(*colors);
        }
    }

    /// Sets whether textbox is hovered.
    pub fn set_hover(&mut self, is_active: bool, position: Option<Position>) {
        self.is_hovered = is_active;

        let colors = self.colors.caption.get(self.is_hovered, self.is_focused);
        self.input.set_accept_button_colors(Some(*colors));
        if !self.is_focused {
            self.input.set_colors(*colors);
        }

        if is_active && let Some(position) = position {
            self.input.highlight_accept_button_in(position.x, position.y);
        } else {
            self.input.highlight_accept_button(false);
        }
    }

    /// Process textbox click.
    pub fn click(&mut self) -> ResponseEvent {
        ResponseEvent::Handled
    }

    /// Draws [`TextBox`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let area = area.inner(Margin::new(5, 0));
        let input_width = area.width.saturating_sub(self.caption_width).min(self.input_width);
        let caption_area = Rect::new(area.x, area.y, self.caption_width, 1);
        let input_area = Rect::new(area.x + self.caption_width, area.y, input_width, 1);

        let colors = self.colors.caption.get(self.is_hovered, self.is_focused);
        let line = Line::styled(format!("  {} ", self.caption), colors);
        frame.render_widget(Paragraph::new(line), caption_area);
        self.input.draw(frame, input_area);

        self.area = area;
        self.area.width = caption_area.width + input_area.width;
    }
}

impl Responsive for TextBox {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let TuiEvent::Key(key) = event {
            match key.code {
                KeyCode::Down | KeyCode::Up | KeyCode::Tab => {
                    return ResponseEvent::NotHandled;
                },
                _ => (),
            }
        }

        let result = self.input.process_event(event);
        if result != ResponseEvent::Handled {
            return result;
        }

        if self.prev_value != self.input.value_full() {
            self.prev_value = self.input.value_full().to_owned();
            return ResponseEvent::Changed;
        }

        ResponseEvent::Handled
    }
}
