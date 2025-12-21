use b4n_config::keys::KeyCommand;
use b4n_tui::utils::{center_horizontal, get_proportional_width};
use b4n_tui::widgets::Select;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::PatternsList;

const HISTORY_SIZE: usize = 20;
const NOT_FOUND_HINT: &str = " No matches found.";

/// Search widget for TUI.
pub struct Search {
    pub is_visible: bool,
    app_data: SharedAppData,
    worker: Option<SharedBgWorker>,
    patterns: Select<PatternsList>,
    matches: Option<usize>,
    width: u16,
    hint: String,
}

impl Search {
    /// Creates new [`Search`] instance.
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        let colors = app_data.borrow().theme.colors.search.clone();
        let patterns = Select::new(PatternsList::default(), colors, false, true).with_prompt(" ");
        let enter = app_data.get_key(KeyCommand::NavigateInto).to_string().to_ascii_uppercase();
        let next = app_data.get_key(KeyCommand::MatchNext).to_string().to_ascii_uppercase();
        let prev = app_data.get_key(KeyCommand::MatchPrevious).to_string().to_ascii_uppercase();

        Self {
            is_visible: false,
            app_data,
            worker,
            patterns,
            matches: None,
            width,
            hint: format!(" {enter} to accept, {next} and {prev} to navigate."),
        }
    }

    /// Returns the search value.
    pub fn value(&self) -> &str {
        self.patterns.value()
    }

    /// Marks [`Search`] as visible.
    pub fn show(&mut self) {
        let context = self.app_data.borrow().current.context.clone();
        let key_name = self
            .app_data
            .get_key(KeyCommand::NavigateComplete)
            .to_string()
            .to_ascii_uppercase();
        self.patterns.items = PatternsList::from(
            self.app_data.borrow_mut().history.get_search_history(&context),
            Some(&key_name),
        );
        self.patterns.update_items_filter();
        self.patterns.set_colors(self.app_data.borrow().theme.colors.search.clone());
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

        let width = get_proportional_width(area.width, self.width, true);
        let area = center_horizontal(area, width, self.patterns.get_screen_height());

        let colors = &self.app_data.borrow().theme.colors.search;
        Self::clear_area(frame, area, colors.normal.bg);
        if area.top() > 0 {
            let area = Rect::new(area.x, area.y.saturating_sub(1), area.width, 1);
            Self::clear_area(frame, area, colors.header.unwrap_or_default().bg);
            self.draw_header(frame, area);
        }

        self.patterns.draw(frame, area.inner(Margin::new(1, 0)));
    }

    fn clear_area(frame: &mut ratatui::Frame<'_>, area: Rect, color: Color) {
        let block = Block::new().style(Style::default().bg(color));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }

    fn draw_header(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let header = &self.app_data.borrow().theme.colors.search.header.unwrap_or_default();
        let area = area.inner(Margin::new(1, 0));

        if let Some(matches) = self.matches {
            let text = format!(" Total matches: {matches}");
            frame.render_widget(Paragraph::new(text).style(header), area);
        } else if self.patterns.value().is_empty() {
            frame.render_widget(Paragraph::new(self.hint.as_str()).style(header), area);
        } else {
            frame.render_widget(Paragraph::new(NOT_FOUND_HINT).style(header), area);
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

    fn complete_with_selected_item(&mut self) {
        if let Some(pattern) = self.patterns.items.get_highlighted_item_name().map(String::from) {
            self.patterns.set_value(pattern);
        }
    }
}

impl Responsive for Search {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if self.app_data.has_binding(event, KeyCommand::SearchReset) && !self.patterns.value().is_empty() {
            self.patterns.reset();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack)
            || event.is_out(MouseEventKind::LeftClick, self.patterns.area)
            || event.is_mouse(MouseEventKind::RightClick)
        {
            self.is_visible = false;
            return ResponseEvent::Handled;
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
            self.is_visible = false;
            self.remember_pattern();

            return ResponseEvent::Handled;
        }

        self.patterns.process_event(event);

        ResponseEvent::Handled
    }
}
