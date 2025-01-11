use crossterm::event::{Event, KeyEvent};
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
    pub fn new<S: Into<Style>>(style: S, show_cursor: bool) -> Self {
        Self {
            input: Default::default(),
            style: style.into(),
            prompt: None,
            show_cursor,
        }
    }

    /// Adds a prompt to the [`Input`] instance.
    pub fn with_prompt<S: Into<Style>>(mut self, prompt: String, style: S) -> Self {
        self.prompt = Some((prompt, style.into()));
        self
    }

    /// Sets the prompt and its style.
    pub fn set_prompt<S: Into<Style>>(&mut self, prompt: Option<(String, S)>) {
        self.prompt = prompt.map(|p| (p.0, p.1.into()));
    }

    /// Sets the input style.
    pub fn set_style<S: Into<Style>>(&mut self, style: S) {
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

    /// Resets the input value.
    pub fn reset(&mut self) {
        self.input.reset();
    }

    /// Draws [`Input`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(self.prompt.as_ref().map(|p| p.0.len() + 1).unwrap_or(1) as u16),
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
        self.input.handle_event(&Event::Key(key));
        ResponseEvent::Handled
    }
}
