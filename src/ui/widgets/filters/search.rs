use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    core::{SharedAppData, SharedBgWorker},
    ui::{ResponseEvent, Responsive, Table, utils::center_horizontal, widgets::Select},
};

use super::PatternsList;

const HISTORY_SIZE: usize = 20;

/// Search widget for TUI.
pub struct Search {
    pub is_visible: bool,
    app_data: SharedAppData,
    worker: Option<SharedBgWorker>,
    patterns: Select<PatternsList>,
    current: String,
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
            current: String::new(),
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
        self.current = String::new();
    }

    /// Draws [`Search`] on the provided frame area.
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

    fn remember_pattern(&mut self) {
        let pattern = self.patterns.value();
        self.current = pattern.to_owned();
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
