use b4n_config::themes::{TextBoxModalColors, TextColors};
use crossterm::event::KeyCode;
use ratatui_core::layout::{Margin, Position, Rect};
use ratatui_core::terminal::Frame;
use ratatui_core::text::{Line, Span};
use ratatui_widgets::paragraph::Paragraph;

use crate::widgets::input::SharedClipboard;
use crate::widgets::{Input, InputValidator, ValidatorKind};
use crate::{ResponseEvent, Responsive, TuiEvent};

/// UI `TextBox`.
pub struct TextBox {
    pub id: usize,
    caption: String,
    input: Input,
    validators: Vec<InputValidator>,
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
    pub fn new(id: usize, caption: impl Into<String>, input_width: u16, colors: TextBoxModalColors) -> Self {
        let caption = caption.into();
        let caption_width = u16::try_from(caption.chars().count()).unwrap_or_default() + 4;
        let mut input = Input::new(colors.caption.normal).with_show_accept_button_on_errors(true);
        input.set_cursor_colors(colors.cursor);

        Self {
            id,
            caption,
            input,
            validators: Vec::new(),
            prev_value: String::new(),
            is_hovered: false,
            is_focused: false,
            show_cursor: colors.cursor.is_some(),
            colors,
            area: Rect::default(),
            caption_width,
            input_width,
        }
    }

    /// Sets initial value for the textbox.
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.input.set_value(value);
        self.validate(true);
        self.prev_value = self.input.value().to_string();
        self
    }

    /// Adds button to the textbox.
    pub fn with_button(mut self, icon: &'static str, action: &'static str) -> Self {
        let response = ResponseEvent::Action(action);
        self.input
            .set_accept_button(Some((icon, response)), Some(self.colors.caption.normal));
        self
    }

    /// Adds clipboard functionality to the textbox.
    pub fn with_clipboard(mut self, clipboard: Option<SharedClipboard>) -> Self {
        self.input.set_clipboard(clipboard);
        self
    }

    /// Adds validator to the textbox.
    pub fn with_validator(mut self, validator: ValidatorKind) -> Self {
        self.validators.push(InputValidator::new(validator));
        self.validate(true);
        self
    }

    /// Sets new caption for the textbox.
    pub fn set_caption(&mut self, caption: impl Into<String>) {
        self.caption = caption.into();
        self.caption_width = u16::try_from(self.caption.chars().count()).unwrap_or_default() + 4;
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

    /// Sets new value for the textbox.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input.set_value(value);
        self.validate(true);
        self.prev_value = self.input.value().to_string();
    }

    /// Returns `true` if provided `x` and `y` are inside the textbox.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.area.contains(Position::new(x, y))
    }

    /// Returns `true` if textbox has error.
    pub fn has_error(&self) -> bool {
        self.input.has_error()
    }

    /// Returns `true` if textbox is focused.
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Activates or deactivates textbox.
    pub fn set_focus(&mut self, is_active: bool) {
        self.is_focused = is_active;
        self.input.show_cursor(is_active && self.show_cursor);
        self.update_input_colors();
    }

    /// Sets whether textbox is hovered.
    pub fn set_hover(&mut self, is_active: bool, position: Option<Position>) {
        self.is_hovered = is_active;
        self.update_input_colors();

        if is_active && let Some(position) = position {
            self.input.highlight_accept_button_in(position.x, position.y);
        } else {
            self.input.highlight_accept_button(false);
        }
    }

    /// Validates textbox using associated validators.
    fn validate(&mut self, force_validation: bool) -> bool {
        if !self.validators.is_empty() && (force_validation || self.prev_value != self.input.value()) {
            self.input.set_error(None);

            let value = self.input.value();
            for validator in &mut self.validators {
                if let Err(error_index) = validator.validate(value) {
                    self.input.set_error(Some(error_index));
                    self.update_input_colors();

                    return true;
                }
            }

            self.update_input_colors();
        }

        self.input.has_error()
    }

    /// Process textbox click.
    pub fn click(&mut self, position: Option<Position>) -> ResponseEvent {
        if let Some(position) = position {
            let response = self.input.process_event(&TuiEvent::click(position));
            if response != ResponseEvent::NotHandled {
                if matches!(response, ResponseEvent::Action(_)) {
                    self.input.highlight_accept_button(false);
                }

                return response;
            }
        }

        ResponseEvent::Handled
    }

    /// Draws [`TextBox`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let area = area.inner(Margin::new(5, 0));
        let input_width = area.width.saturating_sub(self.caption_width).min(self.input_width);
        let caption_area = Rect::new(area.x, area.y, self.caption_width, 1);
        let input_area = Rect::new(area.x + self.caption_width, area.y, input_width, 1);

        let colors = *self.colors.caption.get(self.is_hovered, self.is_focused);
        let spans = vec![self.get_icon(&colors), Span::styled(&self.caption, &colors)];
        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line), caption_area);
        self.input.draw(frame, input_area);

        self.area = area;
        self.area.width = caption_area.width + input_area.width;
    }

    fn update_input_colors(&mut self) {
        let mut input_colors = *self.colors.input.get(self.is_hovered, self.is_focused);
        if self.is_focused {
            input_colors.bg = self.colors.input.focused.bg;
        }

        if self.input.has_error()
            && let Some(colors) = self.colors.error
        {
            input_colors.fg = colors.fg;
            input_colors.dim = colors.dim;
        }

        let mut button_colors = *self.colors.caption.get(self.is_hovered, self.is_focused);
        button_colors.bg = input_colors.bg;

        self.input.set_colors(input_colors);
        self.input.set_accept_button_colors(Some(button_colors));
    }

    fn get_icon(&self, colors: &TextColors) -> Span<'_> {
        if self.input.has_error()
            && let Some(error_colors) = self.colors.error
        {
            let colors = TextColors {
                fg: error_colors.fg,
                dim: error_colors.dim,
                bg: colors.bg,
            };
            Span::styled("  ", &colors)
        } else {
            Span::styled("  ", colors)
        }
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

        if event.is_mouse(crate::MouseEventKind::LeftClick) {
            self.input.highlight_accept_button(false);
        }

        let result = self.input.process_event(event);
        self.validate(false);

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
