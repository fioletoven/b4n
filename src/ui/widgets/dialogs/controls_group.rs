use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::ui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};

use super::{Button, CheckBox};

/// Events used to handle press and focus actions.
#[derive(PartialEq)]
enum ControlEvent {
    None,
    FocusPrev,
    FocusNext,
    Pressed,
}

/// Represents group of the controls in UI.
pub struct ControlsGroup {
    pub inputs: Vec<CheckBox>,
    pub buttons: Vec<Button>,
    focused: usize,
}

impl ControlsGroup {
    /// Creates new [`ControlsGroup`] instance.
    pub fn new(buttons: Vec<Button>) -> Self {
        Self {
            inputs: Vec::new(),
            buttons,
            focused: 0,
        }
    }

    /// Returns result for the control under provided index.
    pub fn result(&self, index: usize) -> ResponseEvent {
        if self.buttons.is_empty() {
            return ResponseEvent::NotHandled;
        }

        self.buttons[index].result()
    }

    /// Focus control under provided index.
    pub fn focus(&mut self, index: usize) {
        if !self.buttons.is_empty() {
            self.buttons[self.focused].set_focus(false);
            self.focused = index.clamp(0, self.buttons.len() - 1);
            self.buttons[self.focused].set_focus(true);
        }
    }

    /// Draws [`ControlsGroup`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
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

        for (i, btn) in self.buttons.iter_mut().enumerate() {
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

impl Responsive for ControlsGroup {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.buttons.is_empty() {
            return ResponseEvent::NotHandled;
        }

        if let TuiEvent::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::LeftClick
        {
            for btn in &self.buttons {
                if btn.contains(mouse.column, mouse.row) {
                    return btn.result();
                }
            }
        }

        let event = map_to_button_event(event);
        if event == ControlEvent::Pressed {
            return self.buttons[self.focused].result();
        }

        if event == ControlEvent::FocusPrev {
            if self.focused == 0 {
                self.focus_last();
            } else {
                self.focus_prev();
            }
        }

        if event == ControlEvent::FocusNext {
            if self.focused == self.buttons.len() - 1 {
                self.focus_first();
            } else {
                self.focus_next();
            }
        }

        ResponseEvent::Handled
    }
}

fn map_to_button_event(event: &TuiEvent) -> ControlEvent {
    match event {
        TuiEvent::Key(key) => match key.code {
            KeyCode::Tab | KeyCode::Right | KeyCode::Down => ControlEvent::FocusNext,
            KeyCode::Left | KeyCode::Up => ControlEvent::FocusPrev,
            KeyCode::Enter => ControlEvent::Pressed,
            _ => ControlEvent::None,
        },
        _ => ControlEvent::None,
    }
}
