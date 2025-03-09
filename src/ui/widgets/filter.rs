use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    app::SharedAppData,
    ui::{ResponseEvent, Responsive},
    utils::logical_expressions::{ParserError, validate},
};

use super::Input;

#[cfg(test)]
#[path = "./filter.tests.rs"]
mod filter_tests;

/// Filter widget for TUI.
#[derive(Default)]
pub struct Filter {
    pub is_visible: bool,
    app_data: SharedAppData,
    input: Input,
    current: String,
    width: u16,
    last_validated: String,
}

impl Filter {
    /// Creates new [`Filter`] instance.
    pub fn new(app_data: SharedAppData, width: u16) -> Self {
        let input = Input::new(app_data.borrow().config.theme.colors.filter.input, true)
            .with_prompt(" ", app_data.borrow().config.theme.colors.filter.prompt)
            .with_error_colors(app_data.borrow().config.theme.colors.filter.error)
            .with_accent_characters("|+&()");

        Self {
            app_data,
            input,
            width,
            ..Default::default()
        }
    }

    /// Returns the filter value.
    pub fn value(&self) -> &str {
        self.input.value()
    }

    /// Marks [`Filter`] as a visible.
    pub fn show(&mut self) {
        self.input.set_colors(self.app_data.borrow().config.theme.colors.filter.input);
        self.input
            .set_prompt_colors(self.app_data.borrow().config.theme.colors.filter.prompt);
        self.input
            .set_error_colors(self.app_data.borrow().config.theme.colors.filter.error);
        self.is_visible = true;
    }

    /// Resets the filter value.
    pub fn reset(&mut self) {
        self.input.reset();
        self.current = String::new();
    }

    /// Draws [`Filter`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let colors = &self.app_data.borrow().config.theme.colors.filter;
        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let area = center(area, width);
        let block = Block::new().style(Style::default().bg(colors.input.bg));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        self.input.draw(frame, area);
    }

    /// Validates the filter value as a logical expression.
    fn validate(&mut self) {
        if self.last_validated == self.input.value() {
            return;
        }

        if let Err(error) = validate(self.input.value()) {
            match error {
                ParserError::ExpectedOperator(index) => self.input.set_error(Some(index)),
                ParserError::ExpectedValue(index) => self.input.set_error(Some(index)),
                ParserError::UnexpectedClosingBracket(index) => self.input.set_error(Some(index)),
                ParserError::ExpectedClosingBracket(index) => self.input.set_error(Some(index)),
            }
        } else {
            self.input.set_error(None);
        }

        self.last_validated = self.input.value().to_owned();
    }
}

impl Responsive for Filter {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if key.code == KeyCode::Esc {
            self.is_visible = false;
            self.input.set_value(self.current.clone());

            return ResponseEvent::Cancelled;
        }

        if key.code == KeyCode::Enter {
            self.is_visible = false;
            self.current = self.input.value().to_owned();

            return ResponseEvent::Handled;
        }

        self.input.process_key(key);
        self.validate();

        ResponseEvent::Handled
    }
}

/// Centers horizontally a [`Rect`] within another [`Rect`] using the provided width.
pub fn center(area: Rect, width: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center).areas(area);
    let top = if area.height > 2 { (area.height - 2).min(3) } else { 0 };
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(top), Constraint::Length(1)])
        .split(area)[1]
}
