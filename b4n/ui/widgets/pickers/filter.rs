use b4n_common::expr::{ParserError, validate};
use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::widgets::Select;
use b4n_tui::{ResponseEvent, TuiEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::pickers::base::PickerBehaviour;
use crate::ui::widgets::{PatternsList, Picker};

#[cfg(test)]
#[path = "./filter.tests.rs"]
mod filter_tests;

const FILTER_HINT: &str = " Use | for OR, & for AND, ! for NOT, and ( ) to group terms.";
const FILTER_HISTORY_SIZE: usize = 20;

pub type Filter = Picker<FilterBehaviour>;

impl Filter {
    /// Creates new [`Filter`] instance.
    pub fn new(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16) -> Self {
        let behaviour = FilterBehaviour::new(Rc::clone(&app_data));
        Picker::new_picker(app_data, worker, width, behaviour)
    }

    /// Toggles filter pin in shared app data.
    pub fn toggle_pin(&mut self) -> ResponseEvent {
        if !self.value().is_empty() {
            if self.behaviour().app_data.borrow().is_pinned {
                self.behaviour_mut().app_data.borrow_mut().is_pinned = false;
            } else {
                self.behaviour_mut().app_data.borrow_mut().is_pinned = true;
                self.behaviour_mut().app_data.borrow_mut().pinned_filter = self.to_option();
            }
        }

        ResponseEvent::Handled
    }

    /// If current filter is pinned, updates it.
    pub fn update_pinned_filter(&mut self) {
        if self.behaviour().app_data.borrow().is_pinned {
            self.behaviour_mut().app_data.borrow_mut().pinned_filter = self.to_option();
        }
    }

    /// Returns `true` if current filter value is valid.
    pub fn is_valid(&self) -> bool {
        self.behaviour().last_error.is_none()
    }

    /// Returns `true` if this is a `FilterReset` event and filter can be reset.
    pub fn is_reset_filter_event(&self, event: &TuiEvent) -> bool {
        !self.behaviour().app_data.borrow().is_pinned
            && self.behaviour().app_data.has_binding(event, KeyCommand::FilterReset)
            && !self.value().is_empty()
    }
}

pub struct FilterBehaviour {
    app_data: SharedAppData,
    last_validated: String,
    last_error: Option<usize>,
    modified_filter: Option<String>,
}

impl FilterBehaviour {
    pub fn new(app_data: SharedAppData) -> Self {
        Self {
            app_data,
            last_validated: String::new(),
            last_error: None,
            modified_filter: None,
        }
    }
}

impl PickerBehaviour for FilterBehaviour {
    fn prompt(&self) -> &str {
        if self.app_data.borrow().is_pinned {
            self.app_data.borrow().config.symbols.pinned.right
        } else {
            self.app_data.borrow().config.symbols.filtered.right
        }
    }

    fn colors(&self) -> SelectColors {
        self.app_data.borrow().theme.colors.filter.clone()
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

    fn add_item(&self, item: &str) {
        if item.trim().is_empty() || self.modified_filter.as_deref().is_some_and(|p| p == item) {
            return;
        }

        let context = self.app_data.borrow().current.context.clone();
        self.app_data
            .borrow_mut()
            .history
            .put_filter_history_item(&context, item.into(), FILTER_HISTORY_SIZE);
    }

    fn remove_item(&self, item: &str) -> bool {
        let context = self.app_data.borrow().current.context.clone();
        self.app_data
            .borrow_mut()
            .history
            .remove_filter_history_item(&context, item)
            .is_some()
    }

    fn validate(&mut self, value: &str) -> Option<usize> {
        if self.last_validated == value {
            return self.last_error;
        }

        value.clone_into(&mut self.last_validated);
        self.last_error = match validate(value) {
            Err(
                ParserError::ExpectedOperator(i)
                | ParserError::UnexpectedOperator(i)
                | ParserError::ExpectedClosingBracket(i)
                | ParserError::UnexpectedClosingBracket(i),
            ) => Some(i),
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

    fn on_show(&mut self, patterns: &mut Select<PatternsList>) -> bool {
        let context = &self.app_data.borrow().current.context;
        let key_name = self.app_data.get_key_name(KeyCommand::NavigateComplete).to_ascii_uppercase();
        patterns.items = PatternsList::from(self.app_data.borrow().history.filter_history(context), Some(&key_name));

        if self.modified_filter.is_none() && self.app_data.borrow().is_pinned {
            let value = patterns.value().trim();
            if !value.is_empty() && !value.starts_with('(') && !value.ends_with('&') {
                let value = format!("( {value} ) & ");
                patterns.set_value(value.clone());
                self.modified_filter = Some(value);
            }
        }

        true
    }

    fn on_reset(&mut self, patterns: &mut Select<PatternsList>) -> bool {
        if let Some(pattern) = &self.modified_filter
            && patterns.value().len() > pattern.len()
        {
            patterns.set_value(pattern);
            return true;
        }

        patterns.reset();
        true
    }

    fn on_close(&mut self, patterns: &mut Select<PatternsList>, is_cancel: bool) -> bool {
        if !is_cancel
            && let Some(len) = self.modified_filter.as_deref().map(str::len)
            && patterns.value().len() < len
        {
            self.modified_filter = None;
        }

        true
    }

    fn draw_header(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect, style: Style) {
        frame.render_widget(Paragraph::new(FILTER_HINT).style(style), area);
    }

    fn pre_process_event(
        &mut self,
        event: &TuiEvent,
        patterns: &mut Select<PatternsList>,
        app_data: &SharedAppData,
    ) -> ResponseEvent {
        if app_data.has_binding(event, KeyCommand::FilterPin) {
            let is_pinned = !app_data.borrow().is_pinned;
            app_data.borrow_mut().is_pinned = is_pinned;
            patterns.set_prompt(self.prompt());

            return ResponseEvent::Handled;
        }

        ResponseEvent::NotHandled
    }
}
