use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::utils::{self, center_horizontal, get_proportional_width};
use b4n_tui::widgets::Select;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::PatternsList;

const HISTORY_SIZE: usize = 20;

/// Defines the varying behaviour between different pickers.
pub trait PickerBehaviour {
    /// Gets prompt shown in the input.
    fn prompt(&self) -> &str;

    /// Gets colors to use in the picker.
    fn colors(&self) -> &SelectColors;

    /// Gets optional accent characters for the input.
    fn accent_characters(&self) -> Option<&str> {
        None
    }

    /// Gets the key command used for `reset` action.
    fn reset_key_command(&self) -> KeyCommand;

    /// Gets response event when back/cancel is triggered.
    fn cancel_response(&self) -> ResponseEvent;

    /// Loads items when the picker is shown.
    fn load_items(&self, app_data: &SharedAppData) -> PatternsList;

    /// Persists all items back to the history file.
    fn save_items(&self, app_data: &SharedAppData, items: &PatternsList);

    /// Validates the current input value.\
    /// Returns `Some(index)` for error position, `None` if valid.
    fn validate(&self, _value: &str) -> Option<usize> {
        None
    }

    /// Gets cancel behaviour. Value indicates whether pressing back/escape should restore the previous value.
    /// If false, the current value is kept.
    fn restores_on_cancel(&self) -> bool {
        false
    }

    /// Gets value indicating whether the dialog should block confirm when validation fails.
    fn blocks_on_error(&self) -> bool {
        false
    }

    /// Called before drawing.
    fn on_draw(&mut self, _patterns: &mut Select<PatternsList>, _area: Rect) {}

    /// Draws the header area.
    fn draw_header(&self, _frame: &mut ratatui::Frame<'_>, _area: Rect, _style: Style) {}
}

pub struct Picker<B: PickerBehaviour> {
    pub is_visible: bool,
    app_data: SharedAppData,
    worker: Option<SharedBgWorker>,
    patterns: Select<PatternsList>,
    current: String,
    last_validated: String,
    width: u16,
    behaviour: B,
}

impl<B: PickerBehaviour> Picker<B> {
    /// Creates new [`Picker`] instance.
    pub fn new_picker(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16, behaviour: B) -> Self {
        let colors = behaviour.colors().clone();
        let mut select = Select::new(PatternsList::default(), colors, false, true).with_prompt(behaviour.prompt());

        if let Some(accents) = behaviour.accent_characters() {
            select = select.with_accent_characters(accents);
        }

        Self {
            is_visible: false,
            app_data,
            worker,
            patterns: select,
            current: String::new(),
            last_validated: String::new(),
            width,
            behaviour,
        }
    }

    /// Marks the picker as visible and loads items.
    pub fn show(&mut self) {
        self.patterns.items = self.behaviour.load_items(&self.app_data);
        self.patterns.update_items_filter();
        self.patterns.set_colors(self.behaviour.colors().clone());
        self.is_visible = true;
    }

    /// Returns the current input value.
    pub fn value(&self) -> &str {
        self.patterns.value()
    }

    /// Sets the input value.
    pub fn set_value(&mut self, value: String) {
        self.patterns.set_value(value.clone());
        self.current = value;
        self.run_validation();
    }

    /// Resets the input value to empty.
    pub fn reset(&mut self) {
        self.patterns.reset();
        self.current = String::new();
    }

    /// Returns picker behaviour.
    pub fn behaviour(&self) -> &B {
        &self.behaviour
    }

    /// Returns mutable picker behaviour.
    pub fn behaviour_mut(&mut self) -> &mut B {
        &mut self.behaviour
    }

    /// Draws the picker on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let width = get_proportional_width(area.width, self.width, true);
        let area = center_horizontal(area, width, self.patterns.get_screen_height());

        self.behaviour.on_draw(&mut self.patterns, area);

        let colors = self.behaviour.colors();
        utils::clear_area(frame, area, colors.normal.bg);
        if area.top() > 0 {
            let header_area = Rect::new(area.x, area.y.saturating_sub(1), area.width, 1);
            let header_style = colors.header.unwrap_or_default();
            utils::clear_area(frame, header_area, header_style.bg);
            self.behaviour
                .draw_header(frame, header_area.inner(Margin::new(1, 0)), (&header_style).into());
        }

        self.patterns.draw(frame, area.inner(Margin::new(1, 0)));
    }

    fn run_validation(&mut self) {
        if self.last_validated == self.patterns.value() {
            return;
        }

        let error_pos = self.behaviour.validate(self.patterns.value());
        self.patterns.set_error(error_pos);
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
        self.behaviour.save_items(&self.app_data, &self.patterns.items);

        if let Some(worker) = &self.worker {
            worker.borrow_mut().save_history(self.app_data.borrow().history.clone());
        }
    }

    fn complete_with_selected_item(&mut self) {
        if let Some(pattern) = self.patterns.items.get_highlighted_item_name().map(String::from) {
            if self.behaviour.validate(self.patterns.value()).is_none() {
                self.last_validated.clone_from(&pattern);
            }

            self.patterns.set_value(pattern);
        }
    }

    fn insert_from_clipboard(&mut self) -> ResponseEvent {
        let text = self.app_data.borrow_mut().clipboard.as_mut().and_then(|c| c.get_text().ok());
        if let Some(text) = text {
            self.patterns.insert_value(&text);
            self.run_validation();
        }

        ResponseEvent::Handled
    }
}

impl<B: PickerBehaviour> Responsive for Picker<B> {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if !self.is_visible {
            return ResponseEvent::NotHandled;
        }

        if self.app_data.has_binding(event, self.behaviour.reset_key_command()) && !self.patterns.value().is_empty() {
            self.patterns.reset();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateDelete) {
            if self.patterns.items.remove_highlighted() {
                self.remember_all_patterns();
            }

            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack)
            || event.is_out(MouseEventKind::LeftClick, self.patterns.area())
        {
            self.is_visible = false;
            if self.behaviour.restores_on_cancel() {
                self.patterns.set_value(self.current.clone());
            }

            return self.behaviour.cancel_response();
        }

        if let Some(line) = event.get_line_no(MouseEventKind::LeftClick, KeyModifiers::NONE, self.patterns.items_area()) {
            self.patterns.items.highlight_item_by_line(line);
            self.complete_with_selected_item();
            self.is_visible = false;
            self.remember_pattern();

            return ResponseEvent::Handled;
        }

        if event.is_mouse(MouseEventKind::RightClick) {
            return self.insert_from_clipboard();
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateComplete) {
            self.complete_with_selected_item();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateInto) {
            if self.behaviour.blocks_on_error() && self.patterns.has_error() {
                return ResponseEvent::Handled;
            }

            self.is_visible = false;
            self.remember_pattern();

            return ResponseEvent::Handled;
        }

        self.patterns.process_event(event);
        self.run_validation();

        ResponseEvent::Handled
    }
}
