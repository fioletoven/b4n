use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Position, Rect},
    widgets::{Block, Widget},
};
use tui_input::backend::crossterm::EventHandler;

use crate::ui::{ResponseEvent, Responsive, TuiEvent, colors::TextColors};

/// Indicates how errors should be highlighted in the input field.
#[derive(Default, PartialEq)]
pub enum ErrorHighlightMode {
    #[default]
    PromptAndIndex,
    Value,
}

/// Input widget for TUI.
#[derive(Default)]
pub struct Input {
    input: tui_input::Input,
    colors: TextColors,
    prompt: Option<(String, TextColors)>,
    error: Option<TextColors>,
    error_index: Option<usize>,
    error_mode: ErrorHighlightMode,
    accent_chars: Option<String>,
    show_cursor: bool,
    position: Position,
}

impl Input {
    /// Creates new [`Input`] instance.
    pub fn new(colors: TextColors, show_cursor: bool) -> Self {
        Self {
            colors,
            show_cursor,
            ..Default::default()
        }
    }

    /// Adds a prompt to the [`Input`] instance.
    pub fn with_prompt(mut self, prompt: impl Into<String>, colors: TextColors) -> Self {
        self.prompt = Some((prompt.into(), colors));
        self
    }

    /// Adds error colors to the [`Input`] instance.
    pub fn with_error_colors(mut self, colors: Option<TextColors>) -> Self {
        self.error = colors;
        self
    }

    /// Adds a set of characters that should be accented by the [`Input`] instance.
    pub fn with_accent_characters(mut self, highlight: impl Into<String>) -> Self {
        self.accent_chars = Some(highlight.into());
        self
    }

    /// Sets the prompt and its colors.
    pub fn set_prompt<S: Into<String>>(&mut self, prompt: Option<(S, TextColors)>) {
        self.prompt = prompt.map(|p| (p.0.into(), p.1));
    }

    /// Sets prompt colors.\
    /// **Note** that it takes effect only if the prompt was already set.
    pub fn set_prompt_colors(&mut self, colors: TextColors) {
        if let Some(prompt) = &mut self.prompt {
            prompt.1 = colors;
        }
    }

    /// Sets the prompt text.\
    /// **Note** that it takes effect only if the prompt was already set.
    pub fn set_prompt_text(&mut self, text: impl Into<String>) {
        if let Some(prompt) = &mut self.prompt {
            prompt.0 = text.into();
        }
    }

    /// Gets the prompt text.
    pub fn prompt(&self) -> Option<&str> {
        if let Some(prompt) = &self.prompt {
            Some(prompt.0.as_str())
        } else {
            None
        }
    }

    /// Sets characters that should be accented by the [`Input`] instance.
    pub fn set_accent_characters(&mut self, highlight: Option<String>) {
        self.accent_chars = highlight;
    }

    /// Sets input colors.
    pub fn set_colors(&mut self, colors: TextColors) {
        self.colors = colors;
    }

    /// Sets whether to show the cursor.
    pub fn set_cursor(&mut self, show_cursor: bool) {
        self.show_cursor = show_cursor;
    }

    /// Returns `true` if cursor is visible.
    pub fn is_cursor_visible(&self) -> bool {
        self.show_cursor
    }

    /// Sets error colors.
    pub fn set_error_colors(&mut self, colors: Option<TextColors>) {
        self.error = colors;
    }

    /// Sets error highlight mode.
    pub fn set_error_mode(&mut self, mode: ErrorHighlightMode) {
        self.error_mode = mode;
    }

    /// Sets error position.
    pub fn set_error(&mut self, error_index: Option<usize>) {
        self.error_index = error_index;
    }

    /// Returns `true` if the input has an error set.
    pub fn has_error(&self) -> bool {
        self.error_index.is_some()
    }

    /// Returns the input value.
    pub fn value(&self) -> &str {
        self.input.value()
    }

    /// Sets the input value.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input = tui_input::Input::new(value.into());
        self.error_index = None;
    }

    /// Resets the input value.
    pub fn reset(&mut self) {
        self.input.reset();
        self.error_index = None;
    }

    /// Draws [`Input`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        frame.render_widget(Block::new().style(&self.colors), area);
        frame.render_widget(&mut *self, area);

        if self.show_cursor {
            frame.set_cursor_position((self.position.x, self.position.y));
        }
    }

    fn render_prompt(&self, x: u16, y: u16, max_x: u16, buf: &mut ratatui::prelude::Buffer) -> u16 {
        let mut count = 0;
        if let Some(prompt) = &self.prompt {
            for (i, char) in prompt.0.chars().enumerate() {
                let x = x + u16::try_from(i).unwrap_or(0);
                if x >= max_x {
                    break;
                }

                count = u16::try_from(i).unwrap_or(0) + 1;

                if self.error_mode == ErrorHighlightMode::PromptAndIndex
                    && self.error_index.is_some()
                    && let Some(colors) = self.error
                {
                    buf[(x, y)].set_char(char).set_fg(colors.fg).set_bg(colors.bg);
                    continue;
                }

                buf[(x, y)].set_char(char).set_fg(prompt.1.fg).set_bg(prompt.1.bg);
            }
        }

        count
    }

    fn render_input(&self, x: u16, y: u16, max_x: u16, scroll: usize, buf: &mut ratatui::prelude::Buffer) {
        if max_x == 0 {
            return;
        }

        for (i, char) in self.input.value().chars().skip(scroll).enumerate() {
            let x = x + u16::try_from(i).unwrap_or(0);
            if x >= max_x {
                return;
            }

            if self
                .error_index
                .is_some_and(|p| self.error_mode == ErrorHighlightMode::Value || p - scroll == i)
                && let Some(colors) = self.error
            {
                buf[(x, y)].set_char(char).set_fg(colors.fg).set_bg(colors.bg);
                continue;
            }

            if self.accent_chars.as_deref().is_some_and(|a| a.contains(char)) {
                buf[(x, y)].set_char(char).set_fg(self.colors.dim).set_bg(self.colors.bg);
            } else {
                buf[(x, y)].set_char(char).set_fg(self.colors.fg).set_bg(self.colors.bg);
            }
        }
    }
}

impl Responsive for Input {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        match event {
            TuiEvent::Key(key) => {
                if key.code == KeyCode::Esc {
                    return ResponseEvent::Cancelled;
                }

                if key.code == KeyCode::Enter {
                    return ResponseEvent::Accepted;
                }

                if key.code == KeyCode::Delete && key.modifiers == KeyModifiers::CONTROL {
                    self.reset();
                    return ResponseEvent::Handled;
                }

                self.input.handle_event(&Event::Key((*key).into()));

                ResponseEvent::Handled
            },
            TuiEvent::Mouse(_) => ResponseEvent::NotHandled,
        }
    }
}

impl Widget for &mut Input {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        if area.width <= 2 {
            return;
        }

        let x = area.left();
        let y = area.top();

        buf[(x, y)].set_char(' ').set_fg(self.colors.fg).set_bg(self.colors.bg);

        let max_x = area.left() + area.width - if self.show_cursor { 2 } else { 1 };

        let x = x + 1 + self.render_prompt(x + 1, y, max_x, buf);
        if x >= max_x {
            return;
        }

        let scroll = self.input.visual_scroll(usize::from(max_x - x));
        self.position = Position::new(x + (self.input.visual_cursor().max(scroll) - scroll) as u16, y);
        self.render_input(x, y, max_x, scroll, buf);
    }
}
