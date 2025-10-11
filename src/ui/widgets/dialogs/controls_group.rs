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
    Checked,
}

/// Represents group of the controls in UI.
pub struct ControlsGroup {
    pub inputs: Vec<CheckBox>,
    pub buttons: Vec<Button>,
    focused: usize,
}

impl ControlsGroup {
    /// Creates new [`ControlsGroup`] instance.
    pub fn new(inputs: Vec<CheckBox>, buttons: Vec<Button>) -> Self {
        Self {
            inputs,
            buttons,
            focused: 0,
        }
    }

    /// Returns result for the control under provided index.
    pub fn result(&self, idx: usize) -> ResponseEvent {
        if let (None, Some(idx)) = self.get_index(idx) {
            return self.buttons[idx].result();
        }

        ResponseEvent::NotHandled
    }

    /// Focus control under provided index.
    pub fn focus(&mut self, idx: usize) {
        self.set_focus(self.focused, false);
        let idx = idx.clamp(0, (self.inputs.len() + self.buttons.len()).saturating_sub(1));
        self.set_focus(idx, true);
        self.focused = idx;
    }

    /// Draws [`ControlsGroup`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(2)])
            .split(area);

        self.draw_inputs(frame, layout[1]);
        self.draw_buttons(frame, layout[2]);
    }

    fn draw_inputs(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if self.inputs.is_empty() {
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); self.inputs.len()])
            .split(area);

        for (i, input) in self.inputs.iter_mut().enumerate() {
            input.draw(frame, layout[i]);
        }
    }

    fn draw_buttons(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if self.buttons.is_empty() {
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.get_buttons_constraints())
            .split(area);

        for (i, btn) in self.buttons.iter_mut().enumerate() {
            btn.draw(frame, layout[i + 1]);
        }
    }

    fn focus_first(&mut self) {
        self.focus(0);
    }

    fn focus_prev(&mut self) {
        self.focus(if self.focused > 0 { self.focused - 1 } else { 0 });
    }

    fn focus_next(&mut self) {
        self.focus(std::cmp::min(
            (self.inputs.len() + self.buttons.len()).saturating_sub(1),
            self.focused + 1,
        ));
    }

    fn focus_last(&mut self) {
        self.focus((self.inputs.len() + self.buttons.len()).saturating_sub(1));
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

    /// Returns tuple `(items_index, buttons_index)`.
    fn get_index(&self, index: usize) -> (Option<usize>, Option<usize>) {
        if index < self.buttons.len() {
            return (None, Some(index));
        }

        let index = index.saturating_sub(self.buttons.len());
        if index < self.inputs.len() {
            (Some(index), None)
        } else {
            (None, None)
        }
    }

    fn set_focus(&mut self, idx: usize, is_active: bool) {
        match self.get_index(idx) {
            (Some(idx), None) => self.inputs[idx].set_focus(is_active),
            (None, Some(idx)) => self.buttons[idx].set_focus(is_active),
            _ => (),
        }
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
            for input in &mut self.inputs {
                if input.contains(mouse.column, mouse.row) {
                    return input.click();
                }
            }

            for btn in &self.buttons {
                if btn.contains(mouse.column, mouse.row) {
                    return btn.result();
                }
            }
        }

        let event = map_to_button_event(event);
        if event == ControlEvent::Checked
            && let (Some(idx), None) = self.get_index(self.focused)
        {
            self.inputs[idx].click();
            return ResponseEvent::Handled;
        }

        if event == ControlEvent::Pressed {
            let (inputs, buttons) = self.get_index(self.focused);
            if let Some(idx) = inputs {
                self.inputs[idx].click();
                return ResponseEvent::Handled;
            } else if let Some(idx) = buttons {
                return self.buttons[idx].result();
            }
        }

        if event == ControlEvent::FocusPrev {
            if self.focused == 0 {
                self.focus_last();
            } else {
                self.focus_prev();
            }
        }

        if event == ControlEvent::FocusNext {
            if self.focused == (self.inputs.len() + self.buttons.len()).saturating_sub(1) {
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
            KeyCode::Char(' ') => ControlEvent::Checked,
            _ => ControlEvent::None,
        },
        _ => ControlEvent::None,
    }
}
