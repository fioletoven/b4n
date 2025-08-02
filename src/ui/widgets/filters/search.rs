use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, Paragraph},
};

use crate::{
    core::{SharedAppData, SharedBgWorker},
    ui::{ResponseEvent, Responsive, Table, utils::center_horizontal, widgets::Select},
};

use super::PatternsList;

const HISTORY_SIZE: usize = 20;
const SEARCH_HINT: &str = " Type to search. Hit Enter, then navigate with n and p.";
const NOT_FOUND_HINT: &str = " No matches found.";

/// Search widget for TUI.
pub struct Search {
    pub is_visible: bool,
    app_data: SharedAppData,
    worker: Option<SharedBgWorker>,
    patterns: Select<PatternsList>,
    matches: Option<usize>,
    width: u16,
}

impl Search {
    /// Creates new [`Search`] instance.
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        let colors = app_data.borrow().theme.colors.filter.clone();
        let patterns = Select::new(PatternsList::default(), colors, false, true).with_prompt(" ");

        Self {
            is_visible: false,
            app_data,
            worker,
            patterns,
            matches: None,
            width,
        }
    }

    /// Returns the search value.
    pub fn value(&self) -> &str {
        self.patterns.value()
    }

    /// Marks [`Search`] as visible.
    pub fn show(&mut self) {
        let context = self.app_data.borrow().current.context.clone();
        self.patterns.items = PatternsList::from(self.app_data.borrow_mut().history.get_search_history(&context));
        self.patterns.update_items_filter();
        self.patterns.set_colors(self.app_data.borrow().theme.colors.filter.clone());
        self.is_visible = true;
    }

    /// Resets the Search value.
    pub fn reset(&mut self) {
        self.patterns.reset();
        self.matches = None;
    }

    /// Sets the number of matches to display in the hint.
    pub fn set_matches(&mut self, matches: Option<usize>) {
        self.matches = matches;
    }

    /// Draws [`Search`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let area = center_horizontal(area, width, self.patterns.items.list.len() + 1);

        let colors = &self.app_data.borrow().theme.colors.filter;
        self.clear_area(frame, area, colors.normal.bg);
        if area.top() > 0 {
            let area = Rect::new(area.x, area.y.saturating_sub(1), area.width, 1);
            self.clear_area(frame, area, colors.header.bg);
            self.draw_header(frame, area);
        }

        self.patterns.draw(frame, area);
    }

    fn clear_area(&self, frame: &mut ratatui::Frame<'_>, area: Rect, color: Color) {
        let block = Block::new().style(Style::default().bg(color));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }

    fn draw_header(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let colors = &self.app_data.borrow().theme.colors.filter;
        let area = area.inner(Margin::new(1, 0));

        if let Some(matches) = self.matches {
            let text = format!(" Total matches: {matches}");
            frame.render_widget(Paragraph::new(text).style(&colors.header), area);
        } else if self.patterns.value().is_empty() {
            frame.render_widget(Paragraph::new(SEARCH_HINT).style(&colors.header), area);
        } else {
            frame.render_widget(Paragraph::new(NOT_FOUND_HINT).style(&colors.header), area);
        }
    }

    fn remember_pattern(&mut self) {
        let pattern = self.patterns.value();
        if self.patterns.items.add(pattern.into(), HISTORY_SIZE) {
            let context = self.app_data.borrow().current.context.clone();
            self.app_data
                .borrow_mut()
                .history
                .update_search_history(&context, self.patterns.items.to_vec());

            if let Some(worker) = &self.worker {
                worker.borrow_mut().save_history(self.app_data.borrow().history.clone());
            }
        }
    }
}

impl Responsive for Search {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if key.code == KeyCode::Esc {
            if self.patterns.value().is_empty() {
                self.is_visible = false;
            }

            self.patterns.reset();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Enter {
            self.is_visible = false;
            self.remember_pattern();

            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Tab {
            if let Some(pattern) = self.patterns.items.get_highlighted_item_name().map(String::from) {
                self.patterns.set_value(pattern);
            }

            return ResponseEvent::Handled;
        }

        self.patterns.process_key(key);

        ResponseEvent::Handled
    }
}
