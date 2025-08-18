use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::ui::{KeyCombination, ResponseEvent, Responsive};

use super::Button;

/// Events used to handle press and focus actions.
#[derive(PartialEq)]
enum ButtonEvent {
    None,
    FocusPrev,
    FocusNext,
    Pressed,
}

/// Represents group of the buttons in UI.
pub struct ButtonsGroup {
    pub buttons: Vec<Button>,
    focused: usize,
}

impl ButtonsGroup {
    /// Creates new [`ButtonGroup`] instance.
    pub fn new(buttons: Vec<Button>) -> Self {
        Self { buttons, focused: 0 }
    }

    /// Returns result for the button under provided index.
    pub fn result(&self, index: usize) -> ResponseEvent {
        if self.buttons.is_empty() {
            return ResponseEvent::NotHandled;
        }

        self.buttons[index].result()
    }

    /// Focus button under provided index.
    pub fn focus(&mut self, index: usize) {
        if !self.buttons.is_empty() {
            self.buttons[self.focused].set_focus(false);
            self.focused = index.clamp(0, self.buttons.len() - 1);
            self.buttons[self.focused].set_focus(true);
        }
    }

    /// Draws [`ButtonsGroup`] on the provided frame area.
    pub fn draw(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if self.buttons.is_empty() {
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.get_buttons_constraints())
            .split(layout[1]);

        for (i, btn) in self.buttons.iter().enumerate() {
            btn.draw(frame, layout[i + 1]);
        }
    }

    fn focus_first(&mut self) {
        self.buttons[self.focused].set_focus(false);
        self.focused = 0;
        self.buttons[self.focused].set_focus(true);
    }

    fn focus_prev(&mut self) {
        self.buttons[self.focused].set_focus(false);
        self.focused = if self.focused > 0 { self.focused - 1 } else { 0 };
        self.buttons[self.focused].set_focus(true);
    }

    fn focus_next(&mut self) {
        self.buttons[self.focused].set_focus(false);
        self.focused = std::cmp::min(self.buttons.len() - 1, self.focused + 1);
        self.buttons[self.focused].set_focus(true);
    }

    fn focus_last(&mut self) {
        self.buttons[self.focused].set_focus(false);
        self.focused = self.buttons.len() - 1;
        self.buttons[self.focused].set_focus(true);
    }

    fn get_buttons_constraints(&self) -> Vec<Constraint> {
        let mut constraints: Vec<Constraint> = Vec::with_capacity(self.buttons.len() + 2);
        constraints.push(Constraint::Fill(1));
        for btn in &self.buttons {
            constraints.push(Constraint::Length(btn.len()));
        }

        constraints.push(Constraint::Length(1));

        constraints
    }
}

impl Responsive for ButtonsGroup {
    fn process_key(&mut self, key: KeyCombination) -> ResponseEvent {
        if self.buttons.is_empty() {
            return ResponseEvent::NotHandled;
        }

        let event = map_key_to_event(key);
        if event == ButtonEvent::Pressed {
            return self.buttons[self.focused].result();
        }

        if event == ButtonEvent::FocusPrev {
            if self.focused == 0 {
                self.focus_last();
            } else {
                self.focus_prev();
            }
        }

        if event == ButtonEvent::FocusNext {
            if self.focused == self.buttons.len() - 1 {
                self.focus_first();
            } else {
                self.focus_next();
            }
        }

        ResponseEvent::Handled
    }
}

fn map_key_to_event(key: KeyCombination) -> ButtonEvent {
    match key.code {
        KeyCode::Tab => ButtonEvent::FocusNext,
        KeyCode::Right => ButtonEvent::FocusNext,
        KeyCode::Down => ButtonEvent::FocusNext,
        KeyCode::Left => ButtonEvent::FocusPrev,
        KeyCode::Up => ButtonEvent::FocusPrev,
        KeyCode::Enter => ButtonEvent::Pressed,
        _ => ButtonEvent::None,
    }
}
