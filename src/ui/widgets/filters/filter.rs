use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    app::SharedAppData,
    ui::{ResponseEvent, Responsive, Table, utils::center_horizontal, widgets::Select},
    utils::logical_expressions::{ParserError, validate},
};

use super::PatternsList;

const HISTORY_SIZE: usize = 10;

#[cfg(test)]
#[path = "./filter.tests.rs"]
mod filter_tests;

/// Filter widget for TUI.
#[derive(Default)]
pub struct Filter {
    pub is_visible: bool,
    app_data: SharedAppData,
    patterns: Select<PatternsList>,
    current: String,
    last_validated: String,
    width: u16,
}

impl Filter {
    /// Creates new [`Filter`] instance.
    pub fn new(app_data: SharedAppData, width: u16) -> Self {
        let colors = app_data.borrow().config.theme.colors.filter.clone();
        let patterns = Select::new(PatternsList::default(), colors, false, true)
            .with_prompt(" ")
            .with_accent_characters("|&!()");

        Self {
            app_data,
            patterns,
            width,
            ..Default::default()
        }
    }

    /// Returns the filter value.
    pub fn value(&self) -> &str {
        self.patterns.value()
    }

    /// Marks [`Filter`] as visible.
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Resets the filter value.
    pub fn reset(&mut self) {
        self.patterns.reset();
        self.current = String::new();
    }

    /// Draws [`Filter`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let colors = &self.app_data.borrow().config.theme.colors.filter;
        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let area = center_horizontal(area, width, self.patterns.items.list.len() + 1);
        let block = Block::new().style(Style::default().bg(colors.normal.bg));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        self.patterns.draw(frame, area);
    }

    /// Validates the filter value as a logical expression.
    fn validate(&mut self) {
        if self.last_validated == self.patterns.value() {
            return;
        }

        if let Err(error) = validate(self.patterns.value()) {
            match error {
                ParserError::ExpectedOperator(index) => self.patterns.set_error(Some(index)),
                ParserError::UnexpectedOperator(index) => self.patterns.set_error(Some(index)),
                ParserError::ExpectedClosingBracket(index) => self.patterns.set_error(Some(index)),
                ParserError::UnexpectedClosingBracket(index) => self.patterns.set_error(Some(index)),
            }
        } else {
            self.patterns.set_error(None);
        }

        self.last_validated = self.patterns.value().to_owned();
    }

    fn remember_pattern(&mut self, pattern: String) {
        self.current = pattern.clone();
        if !pattern.is_empty() && !self.patterns.items.contains(&pattern) && self.patterns.items.len() < HISTORY_SIZE {
            self.patterns.items.list.push(pattern.into());
        }
    }
}

impl Responsive for Filter {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if key.code == KeyCode::Esc {
            if self.patterns.value().is_empty() {
                self.is_visible = false;
                self.patterns.set_value(self.current.clone());
                return ResponseEvent::Cancelled;
            } else {
                self.patterns.reset();
                return ResponseEvent::Handled;
            }
        }

        if key.code == KeyCode::Enter {
            if self.patterns.has_error() {
                return ResponseEvent::Handled;
            }

            self.is_visible = false;
            self.remember_pattern(self.patterns.value().to_owned());
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Tab {
            if let Some(pattern) = self.patterns.items.get_highlighted_item_name().map(String::from) {
                self.last_validated = pattern.clone();
                self.patterns.set_value(pattern);
            }

            return ResponseEvent::Handled;
        }

        self.patterns.process_key(key);
        self.validate();

        ResponseEvent::Handled
    }
}
