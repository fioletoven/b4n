use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::widgets::{Button, CheckBox, Selector};
use crate::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};

enum Control {
    CheckBox(Box<CheckBox>),
    Selector(Box<Selector>),
}

impl Control {
    fn set_focus(&mut self, is_active: bool) {
        match self {
            Control::CheckBox(checkbox) => checkbox.set_focus(is_active),
            Control::Selector(selector) => selector.set_focus(is_active),
        }
    }

    fn contains(&self, x: u16, y: u16) -> bool {
        match self {
            Control::CheckBox(checkbox) => checkbox.contains(x, y),
            Control::Selector(selector) => selector.contains(x, y),
        }
    }

    fn click(&mut self) -> ResponseEvent {
        match self {
            Control::CheckBox(checkbox) => checkbox.click(),
            Control::Selector(selector) => selector.click(),
        }
    }
}

/// Represents group of the controls in UI.
pub struct ControlsGroup {
    controls: Vec<Control>,
    buttons: Vec<Button>,
    focused: usize,
}

impl ControlsGroup {
    /// Creates new [`ControlsGroup`] instance.
    pub fn new(buttons: Vec<Button>) -> Self {
        Self {
            controls: Vec::default(),
            buttons,
            focused: 0,
        }
    }

    pub fn add_checkbox(&mut self, checkbox: CheckBox) {
        self.controls.push(Control::CheckBox(Box::new(checkbox)));
    }

    pub fn add_selector(&mut self, selector: Selector) {
        self.controls.push(Control::Selector(Box::new(selector)));
    }

    pub fn checkbox(&self, id: usize) -> Option<&CheckBox> {
        self.controls.iter().find_map(|control| match control {
            Control::CheckBox(cb) if cb.id == id => Some(cb.as_ref()),
            _ => None,
        })
    }

    pub fn selector(&self, id: usize) -> Option<&Selector> {
        self.controls.iter().find_map(|control| match control {
            Control::Selector(sel) if sel.id == id => Some(sel.as_ref()),
            _ => None,
        })
    }

    pub fn controls_len(&self) -> usize {
        self.controls.len()
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
        let idx = idx.clamp(0, (self.controls.len() + self.buttons.len()).saturating_sub(1));
        self.set_focus(idx, true);
        self.focused = idx;
    }

    /// Draws [`ControlsGroup`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(2)])
            .split(area);

        self.draw_controls(frame, layout[1]);
        self.draw_buttons(frame, layout[2]);
    }

    fn draw_controls(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if self.controls.is_empty() {
            return;
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); self.controls.len()])
            .split(area);

        for (i, control) in self.controls.iter_mut().enumerate() {
            match control {
                Control::CheckBox(checkbox) => checkbox.draw(frame, layout[i]),
                Control::Selector(selector) => selector.draw(frame, layout[i]),
            }
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
        self.focus(self.focused.saturating_sub(1));
    }

    fn focus_next(&mut self) {
        self.focus(std::cmp::min(
            (self.controls.len() + self.buttons.len()).saturating_sub(1),
            self.focused + 1,
        ));
    }

    fn focus_last(&mut self) {
        self.focus((self.controls.len() + self.buttons.len()).saturating_sub(1));
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
        if index < self.controls.len() {
            (Some(index), None)
        } else {
            (None, None)
        }
    }

    fn set_focus(&mut self, idx: usize, is_active: bool) {
        match self.get_index(idx) {
            (Some(idx), None) => self.controls[idx].set_focus(is_active),
            (None, Some(idx)) => self.buttons[idx].set_focus(is_active),
            _ => (),
        }
    }

    fn focus_element_at(&mut self, x: u16, y: u16) {
        if let Some(i) = self.buttons.iter().position(|b| b.contains(x, y)) {
            self.set_focus(self.focused, false);
            self.buttons[i].set_focus(true);
            self.focused = i;
            return;
        }

        if let Some(i) = self.controls.iter().position(|i| i.contains(x, y)) {
            self.set_focus(self.focused, false);
            self.controls[i].set_focus(true);
            self.focused = self.buttons.len() + i;
        }
    }
}

impl Responsive for ControlsGroup {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.buttons.is_empty() {
            return ResponseEvent::NotHandled;
        }

        if let TuiEvent::Mouse(mouse) = event {
            if mouse.kind == MouseEventKind::LeftClick {
                for input in &mut self.controls {
                    if input.contains(mouse.column, mouse.row) {
                        return input.click();
                    }
                }

                for btn in &self.buttons {
                    if btn.contains(mouse.column, mouse.row) {
                        return btn.result();
                    }
                }
            } else if mouse.kind == MouseEventKind::Moved {
                self.focus_element_at(mouse.column, mouse.row);
                return ResponseEvent::Handled;
            }
        }

        let event = map_to_button_event(event);
        if event == ControlEvent::Checked
            && let (Some(idx), None) = self.get_index(self.focused)
        {
            self.controls[idx].click();
            return ResponseEvent::Handled;
        }

        if event == ControlEvent::Pressed {
            let (inputs, buttons) = self.get_index(self.focused);
            if let Some(idx) = inputs {
                self.controls[idx].click();
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
            if self.focused == (self.controls.len() + self.buttons.len()).saturating_sub(1) {
                self.focus_first();
            } else {
                self.focus_next();
            }
        }

        ResponseEvent::Handled
    }
}

/// Events used to handle press and focus actions.
#[derive(PartialEq)]
enum ControlEvent {
    None,
    FocusPrev,
    FocusNext,
    Pressed,
    Checked,
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
        TuiEvent::Mouse(_) => ControlEvent::None,
    }
}
