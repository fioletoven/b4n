use crossterm::event::KeyModifiers;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    core::{SharedAppData, SharedAppDataExt, SharedBgWorker},
    ui::{KeyCommand, MouseEventKind, ResponseEvent, Responsive, Table, TuiEvent, utils::center_horizontal, widgets::Select},
    utils::logical_expressions::{ParserError, validate},
};

use super::PatternsList;

const HISTORY_SIZE: usize = 20;

#[cfg(test)]
#[path = "./filter.tests.rs"]
mod filter_tests;

/// Filter widget for TUI.
pub struct Filter {
    pub is_visible: bool,
    app_data: SharedAppData,
    worker: Option<SharedBgWorker>,
    patterns: Select<PatternsList>,
    current: String,
    last_validated: String,
    width: u16,
}

impl Filter {
    /// Creates new [`Filter`] instance.
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        let colors = app_data.borrow().theme.colors.filter.clone();
        let patterns = Select::new(PatternsList::default(), colors, false, true)
            .with_prompt(" ")
            .with_accent_characters("|&!()");

        Self {
            is_visible: false,
            app_data,
            worker,
            patterns,
            current: String::new(),
            last_validated: String::new(),
            width,
        }
    }

    /// Returns the filter value.
    pub fn value(&self) -> &str {
        self.patterns.value()
    }

    /// Marks [`Filter`] as visible.
    pub fn show(&mut self) {
        let context = self.app_data.borrow().current.context.clone();
        let key_name = self
            .app_data
            .get_key(KeyCommand::NavigateComplete)
            .to_string()
            .to_ascii_uppercase();
        self.patterns.items = PatternsList::from(
            self.app_data.borrow_mut().history.get_filter_history(&context),
            Some(&key_name),
        );
        self.patterns.update_items_filter();
        self.patterns.set_colors(self.app_data.borrow().theme.colors.filter.clone());
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

        let colors = &self.app_data.borrow().theme.colors.filter;
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

    fn remember_pattern(&mut self) {
        let pattern = self.patterns.value();
        self.current = pattern.to_owned();
        if self.patterns.items.add(pattern.into(), HISTORY_SIZE) {
            let context = self.app_data.borrow().current.context.clone();
            self.app_data
                .borrow_mut()
                .history
                .update_filter_history(&context, self.patterns.items.to_vec());

            if let Some(worker) = &self.worker {
                worker.borrow_mut().save_history(self.app_data.borrow().history.clone());
            }
        }
    }

    fn complete_with_selected_item(&mut self) {
        if let Some(pattern) = self.patterns.items.get_highlighted_item_name().map(String::from) {
            self.last_validated.clone_from(&pattern);
            self.patterns.set_value(pattern);
        }
    }
}

impl Responsive for Filter {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if self.app_data.has_binding(event, KeyCommand::FilterReset) && !self.patterns.value().is_empty() {
            self.patterns.reset();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack)
            || event.is_out(MouseEventKind::LeftClick, self.patterns.area)
        {
            self.is_visible = false;
            self.patterns.set_value(self.current.clone());
            return ResponseEvent::Cancelled;
        }

        if let Some(line) = event.get_clicked_line_no(MouseEventKind::LeftClick, KeyModifiers::NONE, self.patterns.area) {
            self.patterns.items.highlight_item_by_line(line);
            self.complete_with_selected_item();
            self.is_visible = false;
            self.remember_pattern();

            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateComplete) {
            self.complete_with_selected_item();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateInto) {
            if self.patterns.has_error() {
                return ResponseEvent::Handled;
            }

            self.is_visible = false;
            self.remember_pattern();

            return ResponseEvent::Handled;
        }

        self.patterns.process_event(event);
        self.validate();

        ResponseEvent::Handled
    }
}
