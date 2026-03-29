use b4n_common::expr::{ParserError, validate};
use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::ResponseEvent;
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
    colors: SelectColors,
}

impl FilterBehaviour {
    /// Creates new [`FilterBehaviour`] instance.
    pub fn new(app_data: &SharedAppData) -> Self {
        Self {
            hint: " Use | for OR, & for AND, and parentheses to group terms.",
            colors: app_data.borrow().theme.colors.filter.clone(),
        }
    }
}

impl PickerBehaviour for FilterBehaviour {
    fn prompt(&self) -> &str {
        " "
    }

    fn colors(&self) -> &SelectColors {
        &self.colors
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

    fn validate(&self, value: &str) -> Option<usize> {
        match validate(value) {
            Err(ParserError::ExpectedOperator(i))
            | Err(ParserError::UnexpectedOperator(i))
            | Err(ParserError::ExpectedClosingBracket(i))
            | Err(ParserError::UnexpectedClosingBracket(i)) => Some(i),
            _ => None,
        }
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
}
