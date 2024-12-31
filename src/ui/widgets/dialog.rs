use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Clear, Paragraph},
};
use textwrap::Options;

use crate::ui::{colors::TextColors, ResponseEvent, Responsive};

use super::{Button, ButtonsGroup};

/// UI modal dialog
pub struct Dialog {
    pub is_visible: bool,
    width: u16,
    colors: TextColors,
    message: String,
    buttons: ButtonsGroup,
    default_button: usize,
}

impl Dialog {
    /// Creates new [`Dialog`] instance
    pub fn new(message: String, buttons: Vec<Button>, width: u16, colors: TextColors) -> Self {
        let default_button = if buttons.len() > 0 { buttons.len() - 1 } else { 0 };
        let mut buttons = ButtonsGroup::new(buttons);
        buttons.focus(default_button);

        Self {
            is_visible: false,
            width,
            colors,
            message,
            buttons,
            default_button,
        }
    }

    /// Mark [`Dialog`] as a visible
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Draws [`Dialog`] on the provided frame area
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let mut width = std::cmp::min(area.width, self.width);
        if width > 2 {
            width -= 2
        }

        let text = textwrap::wrap(
            &self.message,
            Options::new(width.into()).initial_indent("  ").subsequent_indent("  "),
        );
        let height = text.len() + 4;

        let area = center(area, Constraint::Length(self.width), Constraint::Length(height as u16));
        let block = Block::new().style(Style::default().bg(self.colors.bg));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(3)])
            .split(area);

        let lines: Vec<Line> = text.iter().map(|i| Line::from(i.as_ref())).collect();
        frame.render_widget(Paragraph::new(lines).fg(self.colors.fg), layout[1]);

        self.buttons.draw(frame, layout[2]);
    }
}

impl Responsive for Dialog {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if key.code == KeyCode::Esc {
            self.is_visible = false;
            return self.buttons.result(self.default_button);
        }

        let result = self.buttons.process_key(key);
        if result != ResponseEvent::Handled {
            self.is_visible = false;
        }

        result
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new(String::new(), vec![], 0, TextColors::default())
    }
}

/// Centers a [`Rect`] within another [`Rect`] using the provided [`Constraint`]s.
fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Flex::Center).areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
