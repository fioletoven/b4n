use b4n_common::expr::{ParserError, validate};
use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::widgets::Select;
use b4n_tui::{ResponseEvent, TuiEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::pickers::base::PickerBehaviour;
use crate::ui::widgets::{PatternsList, Picker};

#[cfg(test)]
#[path = "./filter.tests.rs"]
mod filter_tests;

const FILTER_HISTORY_SIZE: usize = 20;

pub type Filter = Picker<FilterBehaviour>;

impl Filter {
    /// Creates new [`Filter`] instance.
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        let behaviour = FilterBehaviour::new(&app_data);
        Picker::new_picker(app_data, worker, width, behaviour)
    }
}

pub struct FilterBehaviour {
    hint: &'static str,
    last_validated: String,
    last_error: Option<usize>,
    is_pinned: bool,
}

impl FilterBehaviour {
    pub fn new(app_data: &SharedAppData) -> Self {
        Self {
            hint: " Use | for OR, & for AND, and parentheses to group terms.",
            last_validated: String::new(),
            last_error: None,
            is_pinned: app_data.borrow().is_pinned,
        }
    }
}

impl PickerBehaviour for FilterBehaviour {
    fn prompt(&self) -> &str {
        if self.is_pinned { "󰐃 " } else { " " }
    }

    fn colors(&self, app_data: &SharedAppData) -> SelectColors {
        app_data.borrow().theme.colors.filter.clone()
    }

    fn accent_characters(&self) -> Option<&str> {
        Some("|&!()")
    }

    fn reset_key_command(&self) -> KeyCommand {
        KeyCommand::FilterReset
    }

    fn cancel_response(&self) -> ResponseEvent {
        ResponseEvent::Cancelled
    }

    fn load_items(&self, app_data: &SharedAppData) -> PatternsList {
        let context = &app_data.borrow().current.context;
        let key_name = app_data.get_key_name(KeyCommand::NavigateComplete).to_ascii_uppercase();
        PatternsList::from(app_data.borrow().history.filter_history(context), Some(&key_name))
    }

    fn add_item(&self, app_data: &SharedAppData, item: &str) {
        let context = app_data.borrow().current.context.clone();
        app_data
            .borrow_mut()
            .history
            .put_filter_history_item(&context, item.into(), FILTER_HISTORY_SIZE);
    }

    fn remove_item(&self, app_data: &SharedAppData, item: &str) -> bool {
        let context = app_data.borrow().current.context.clone();
        app_data
            .borrow_mut()
            .history
            .remove_filter_history_item(&context, item)
            .is_some()
    }

    fn validate(&mut self, value: &str) -> Option<usize> {
        if self.last_validated == value {
            return self.last_error;
        }

        self.last_validated = value.to_owned();
        self.last_error = match validate(value) {
            Err(ParserError::ExpectedOperator(i))
            | Err(ParserError::UnexpectedOperator(i))
            | Err(ParserError::ExpectedClosingBracket(i))
            | Err(ParserError::UnexpectedClosingBracket(i)) => Some(i),
            _ => None,
        };

        self.last_error
    }

    fn restores_on_cancel(&self) -> bool {
        true
    }

    fn blocks_on_error(&self) -> bool {
        true
    }

    fn draw_header(&self, frame: &mut ratatui::Frame<'_>, area: Rect, style: Style) {
        frame.render_widget(Paragraph::new(self.hint).style(style), area);
    }

    fn process_event(
        &mut self,
        event: &TuiEvent,
        patterns: &mut Select<PatternsList>,
        app_data: &SharedAppData,
    ) -> ResponseEvent {
        if app_data.has_binding(event, KeyCommand::FilterPin) {
            self.is_pinned = !app_data.borrow().is_pinned;
            app_data.borrow_mut().is_pinned = self.is_pinned;
            patterns.set_prompt(self.prompt());

            return ResponseEvent::Handled;
        }

        ResponseEvent::NotHandled
    }
}
