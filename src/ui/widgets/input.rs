use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::Paragraph,
};
use tui_input::backend::crossterm::EventHandler;

use crate::ui::{ResponseEvent, Responsive};

/// Input widget for TUI.
#[derive(Default)]
pub struct Input {
    input: tui_input::Input,
    style: Style,
    prompt: Option<(String, Style)>,
    show_cursor: bool,
}

impl Input {
    /// Creates new [`Input`] instance.
    pub fn new(style: impl Into<Style>, show_cursor: bool) -> Self {
        Self {
            input: Default::default(),
            style: style.into(),
            prompt: None,
            show_cursor,
        }
    }

    /// Adds a prompt to the [`Input`] instance.
    pub fn with_prompt(mut self, prompt: impl Into<String>, style: impl Into<Style>) -> Self {
        self.prompt = Some((prompt.into(), style.into()));
        self
    }

    /// Sets the prompt and its style.
    pub fn set_prompt<Str: Into<String>, Sty: Into<Style>>(&mut self, prompt: Option<(Str, Sty)>) {
        self.prompt = prompt.map(|p| (p.0.into(), p.1.into()));
    }

    /// Sets the prompt style.  
    /// **Note** that it takes effect only if the prompt was already set.
    pub fn set_prompt_style(&mut self, style: impl Into<Style>) {
        if let Some(prompt) = &mut self.prompt {
            prompt.1 = style.into();
        }
    }

    /// Sets the prompt text.  
    /// **Note** that it takes effect only if the prompt was already set.
    pub fn set_prompt_text(&mut self, text: impl Into<String>) {
        if let Some(prompt) = &mut self.prompt {
            prompt.0 = text.into();
        }
    }

    /// Sets the input style.
    pub fn set_style(&mut self, style: impl Into<Style>) {
        self.style = style.into();
    }

    /// Sets whether to show the cursor.
    pub fn set_cursor(&mut self, show_cursor: bool) {
        self.show_cursor = show_cursor;
    }

    /// Returns the input value.
    pub fn value(&self) -> &str {
        self.input.value()
    }

    /// Sets the input value.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input = tui_input::Input::new(value.into());
    }

    /// Resets the input value.
    pub fn reset(&mut self) {
        self.input.reset();
    }

    /// Draws [`Input`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(self.prompt.as_ref().map(|p| p.0.chars().count() + 1).unwrap_or(1) as u16),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(area);
        let width = if self.show_cursor {
            layout[1].width.max(1) - 1
        } else {
            layout[1].width
        };

        let scroll = self.input.visual_scroll(width as usize);
        let input = Paragraph::new(self.input.value())
            .style(self.style)
            .scroll((0, scroll as u16));

        if let Some((prompt, style)) = &self.prompt {
            frame.render_widget(Line::from(format!(" {}", prompt)).style(*style), layout[0]);
        }

        frame.render_widget(input, layout[1]);

        if self.show_cursor {
            frame.set_cursor_position((
                layout[1].x + (self.input.visual_cursor().max(scroll) - scroll) as u16,
                layout[1].y,
            ));
        }
    }
}

impl Responsive for Input {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
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

        self.input.handle_event(&Event::Key(key));

        ResponseEvent::Handled
    }
}
