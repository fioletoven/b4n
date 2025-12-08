use b4n_config::themes::TextColors;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph};
use textwrap::Options;

use crate::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, utils::center};

use super::{Button, CheckBox, ControlsGroup};

/// UI modal dialog.
pub struct Dialog {
    pub is_visible: bool,
    width: u16,
    colors: TextColors,
    message: String,
    controls: ControlsGroup,
    default_button: usize,
    area: Rect,
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new(String::new(), Vec::new(), 0, TextColors::default())
    }
}

impl Dialog {
    /// Creates new [`Dialog`] instance.
    pub fn new(message: String, buttons: Vec<Button>, width: u16, colors: TextColors) -> Self {
        let default_button = if buttons.is_empty() { 0 } else { buttons.len() - 1 };
        let mut buttons = ControlsGroup::new(Vec::new(), buttons);
        buttons.focus(default_button);

        Self {
            is_visible: false,
            width,
            colors,
            message,
            controls: buttons,
            default_button,
            area: Rect::default(),
        }
    }

    /// Sets provided inputs for the dialog.
    pub fn with_inputs(mut self, inputs: Vec<CheckBox>) -> Self {
        self.controls.inputs = inputs;
        self
    }

    /// Returns input under specified `id`.
    pub fn input(&self, id: usize) -> Option<&CheckBox> {
        self.controls.inputs.iter().find(|i| i.id == id)
    }

    /// Marks [`Dialog`] as a visible.
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Draws [`Dialog`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let text = textwrap::wrap(
            &self.message,
            Options::new(width.into()).initial_indent("  ").subsequent_indent("  "),
        );
        let lines = u16::try_from(self.controls.inputs.len()).unwrap_or_default();
        let lines = if lines == 0 { 3 } else { lines + 4 };
        let height = u16::try_from(text.len()).unwrap_or_default() + lines + 1;

        self.area = center(area, Constraint::Length(self.width), Constraint::Length(height));
        let block = Block::new().style(Style::default().bg(self.colors.bg));

        frame.render_widget(Clear, self.area);
        frame.render_widget(block, self.area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(lines)])
            .split(self.area);

        let lines: Vec<Line> = text.iter().map(|i| Line::from(i.as_ref())).collect();
        frame.render_widget(Paragraph::new(lines).fg(self.colors.fg), layout[1]);

        self.controls.draw(frame, layout[2]);
    }
}

impl Responsive for Dialog {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if matches!(event, TuiEvent::Key(key) if key.code == KeyCode::Esc) || event.is_out(MouseEventKind::LeftClick, self.area) {
            self.is_visible = false;
            return self.controls.result(self.default_button);
        }

        let result = self.controls.process_event(event);
        if result != ResponseEvent::Handled {
            self.is_visible = false;
        }

        result
    }
}
