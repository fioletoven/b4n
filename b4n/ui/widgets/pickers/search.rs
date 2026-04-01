use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::widgets::Select;
use b4n_tui::{ResponseEvent, table::Table};
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::{PatternsList, Picker, PickerBehaviour};

const SEARCH_HISTORY_SIZE: usize = 20;

pub type Search = Picker<SearchBehaviour>;

impl Search {
    /// Creates new [`Search`] instance.
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        let behaviour = SearchBehaviour::new(Rc::clone(&app_data));
        Picker::new_picker(app_data, worker, width, behaviour)
    }

    /// Highlights item under the specified mouse position on the first search draw.
    pub fn highlight_position(&mut self, position: Option<Position>) {
        self.behaviour_mut().set_highlight_position(position);
    }

    /// Sets the number of matches to display in the hint.
    pub fn set_matches(&mut self, matches: Option<usize>) {
        self.behaviour_mut().set_matches(matches);
    }

    /// Returns the current match count.
    pub fn matches(&self) -> Option<usize> {
        self.behaviour().matches()
    }
}

pub struct SearchBehaviour {
    app_data: SharedAppData,
    hint: String,
    matches: Option<usize>,
    highlight_position: Option<Position>,
}

impl SearchBehaviour {
    /// Creates new [`SearchBehaviour`] instance.
    pub fn new(app_data: SharedAppData) -> Self {
        let enter = app_data.get_key_name(KeyCommand::NavigateInto).to_ascii_uppercase();
        let next = app_data.get_key_name(KeyCommand::MatchNext).to_ascii_uppercase();
        let prev = app_data.get_key_name(KeyCommand::MatchPrevious).to_ascii_uppercase();

        Self {
            app_data,
            hint: format!(" {enter} to accept, {next} and {prev} to navigate."),
            matches: None,
            highlight_position: None,
        }
    }

    /// Sets the number of matches to display in the header.
    pub fn set_matches(&mut self, matches: Option<usize>) {
        self.matches = matches;
    }

    /// Returns the current match count.
    pub fn matches(&self) -> Option<usize> {
        self.matches
    }

    /// Highlights item under the specified mouse position on the next draw.
    pub fn set_highlight_position(&mut self, position: Option<Position>) {
        self.highlight_position = position;
    }
}

impl PickerBehaviour for SearchBehaviour {
    fn prompt(&self) -> &str {
        " "
    }

    fn colors(&self) -> SelectColors {
        self.app_data.borrow().theme.colors.search.clone()
    }

    fn reset_key_command(&self) -> KeyCommand {
        KeyCommand::SearchReset
    }

    fn cancel_response(&self) -> ResponseEvent {
        ResponseEvent::Handled
    }

    fn load_items(&self) -> PatternsList {
        let context = &self.app_data.borrow().current.context;
        let key_name = self.app_data.get_key_name(KeyCommand::NavigateComplete).to_ascii_uppercase();
        PatternsList::from(self.app_data.borrow().history.search_history(context), Some(&key_name))
    }

    fn add_item(&self, item: &str) {
        let context = self.app_data.borrow().current.context.clone();
        self.app_data
            .borrow_mut()
            .history
            .put_search_history_item(&context, item.into(), SEARCH_HISTORY_SIZE);
    }

    fn remove_item(&self, item: &str) -> bool {
        let context = self.app_data.borrow().current.context.clone();
        self.app_data
            .borrow_mut()
            .history
            .remove_search_history_item(&context, item)
            .is_some()
    }

    fn restores_on_cancel(&self) -> bool {
        false
    }

    fn on_draw(&mut self, patterns: &mut Select<PatternsList>, area: Rect) {
        if let Some(position) = self.highlight_position.take()
            && area.contains(position)
        {
            let line = position
                .y
                .saturating_sub(area.y)
                .saturating_sub(u16::from(patterns.is_filter_visible()));
            patterns.items.highlight_item_by_line(line);
        }
    }

    fn draw_header(&self, frame: &mut ratatui::Frame<'_>, area: Rect, style: Style) {
        if let Some(matches) = self.matches {
            let text = format!(" Total matches: {matches}");
            frame.render_widget(Paragraph::new(text).style(style), area);
        } else {
            frame.render_widget(Paragraph::new(self.hint.as_str()).style(style), area);
        }
    }
}
