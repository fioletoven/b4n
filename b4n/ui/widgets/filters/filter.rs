use b4n_common::expr::{ParserError, validate};
use b4n_config::keys::KeyCommand;
use b4n_tui::utils::{self, center_horizontal, get_proportional_width};
use b4n_tui::widgets::Select;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Margin, Rect};
use ratatui::widgets::Paragraph;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::PatternsList;

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
    hint: &'static str,
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
            hint: " Use | for OR, & for AND, and parentheses to group terms.",
        }
    }

    /// Marks [`Filter`] as visible.
    pub fn show(&mut self) {
        let context = self.app_data.borrow().current.context.clone();
        let key_name = self.app_data.get_key_name(KeyCommand::NavigateComplete).to_ascii_uppercase();
        self.patterns.items = PatternsList::from(
            self.app_data.borrow_mut().history.get_filter_history(&context),
            Some(&key_name),
        );
        self.patterns.update_items_filter();
        self.patterns.set_colors(self.app_data.borrow().theme.colors.filter.clone());
        self.is_visible = true;
    }

    /// Returns the filter value.
    pub fn value(&self) -> &str {
        self.patterns.value()
    }

    /// Sets the filter value.
    pub fn set_value(&mut self, value: String) {
        self.patterns.set_value(value.clone());
        self.current = value;
        self.validate();
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

        let width = get_proportional_width(area.width, self.width, true);
        let area = center_horizontal(area, width, self.patterns.get_screen_height());

        let colors = &self.app_data.borrow().theme.colors.filter;
        utils::clear_area(frame, area, colors.normal.bg);
        if area.top() > 0 {
            let area = Rect::new(area.x, area.y.saturating_sub(1), area.width, 1);
            utils::clear_area(frame, area, colors.header.unwrap_or_default().bg);
            frame.render_widget(
                Paragraph::new(self.hint).style(&colors.header.unwrap_or_default()),
                area.inner(Margin::new(1, 0)),
            );
        }

        self.patterns.draw(frame, area.inner(Margin::new(1, 0)));
    }

    /// Validates the filter value as a logical expression.
    fn validate(&mut self) {
        if self.last_validated == self.patterns.value() {
            return;
        }

        if let Err(error) = validate(self.patterns.value()) {
            match error {
                ParserError::ExpectedOperator(index)
                | ParserError::UnexpectedOperator(index)
                | ParserError::ExpectedClosingBracket(index)
                | ParserError::UnexpectedClosingBracket(index) => self.patterns.set_error(Some(index)),
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
            self.remember_all_patterns();
        }
    }

    fn remember_all_patterns(&mut self) {
        let context = self.app_data.borrow().current.context.clone();
        self.app_data
            .borrow_mut()
            .history
            .update_filter_history(&context, self.patterns.items.to_vec());

        if let Some(worker) = &self.worker {
            worker.borrow_mut().save_history(self.app_data.borrow().history.clone());
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
            self.reset();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateDelete) {
            if self.patterns.items.remove_highlighted() {
                self.remember_all_patterns();
            }

            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack)
            || event.is_out(MouseEventKind::LeftClick, self.patterns.area)
            || event.is_out(MouseEventKind::RightClick, self.patterns.area)
        {
            self.is_visible = false;
            self.patterns.set_value(self.current.clone());
            return ResponseEvent::Cancelled;
        }

        if let Some(line) = event.get_line_no(MouseEventKind::LeftClick, KeyModifiers::NONE, self.patterns.area) {
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
