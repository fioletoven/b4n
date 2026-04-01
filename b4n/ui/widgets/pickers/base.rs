use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::utils::{self, center_horizontal, get_proportional_width};
use b4n_tui::widgets::{ErrorHighlightMode, Select};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Style;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::widgets::{PatternItem, PatternsList};

/// Defines the varying behaviour between different pickers.
pub trait PickerBehaviour {
    /// Gets prompt shown in the input.
    fn prompt(&self) -> &str;

    /// Gets colors to use in the picker.
    fn colors(&self) -> SelectColors;

    /// Gets optional accent characters for the input.
    fn accent_characters(&self) -> Option<&str> {
        None
    }

    /// Gets the key command used for `reset` action.
    fn reset_key_command(&self) -> KeyCommand;

    /// Gets response event when back/cancel is triggered.
    fn cancel_response(&self) -> ResponseEvent;

    /// Loads items when the picker is shown.
    fn load_items(&self) -> PatternsList;

    /// Adds an item to the configuration history.
    fn add_item(&self, item: &str);

    /// Removes an item from the configuration history.
    fn remove_item(&self, item: &str) -> bool;

    /// Gets value indicating whether highlighted item can be removed.
    fn can_remove(&self, item: Option<&PatternItem>) -> bool {
        item.is_some()
    }

    /// Gets error highlight mode for the picker input.
    fn error_mode(&self) -> ErrorHighlightMode {
        ErrorHighlightMode::PromptAndIndex
    }

    /// Validates the current input value.\
    /// Returns `Some(index)` for error position, `None` if valid.
    fn validate(&mut self, _value: &str) -> Option<usize> {
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

    /// Gets response that should be returned by the picker on accepting selected item.
    fn navigate_into(&self, _pattern: &str, _highlighted: Option<&str>) -> ResponseEvent {
        ResponseEvent::Handled
    }

    /// Called before drawing.
    fn on_draw(&mut self, _patterns: &mut Select<PatternsList>, _area: Rect) {}

    /// Gets value indicating whether header should be visible.
    fn has_header(&self) -> bool {
        true
    }

    /// Draws the header area.
    fn draw_header(&self, _frame: &mut ratatui::Frame<'_>, _area: Rect, _style: Style) {}

    /// Additional events processing logic.
    fn process_event(
        &mut self,
        _event: &TuiEvent,
        _patterns: &mut Select<PatternsList>,
        _app_data: &SharedAppData,
    ) -> ResponseEvent {
        ResponseEvent::NotHandled
    }
}

pub struct Picker<B: PickerBehaviour> {
    pub is_visible: bool,
    app_data: SharedAppData,
    worker: Option<SharedBgWorker>,
    patterns: Select<PatternsList>,
    current: String,
    width: u16,
    behaviour: B,
}

impl<B: PickerBehaviour> Picker<B> {
    /// Creates new [`Picker`] instance.
    pub fn new_picker(app_data: SharedAppData, worker: Option<SharedBgWorker>, width: u16, behaviour: B) -> Self {
        let mut select = Select::new(PatternsList::default(), behaviour.colors(), false, true).with_prompt(behaviour.prompt());

        if let Some(accents) = behaviour.accent_characters() {
            select = select.with_accent_characters(accents);
        }

        select.set_error_mode(behaviour.error_mode());

        Self {
            is_visible: false,
            app_data,
            worker,
            patterns: select,
            current: String::new(),
            width,
            behaviour,
        }
    }

    /// Marks the picker as visible and loads items.
    pub fn show(&mut self) {
        self.patterns.items = self.behaviour.load_items();
        self.patterns.update_items_filter();
        self.patterns.set_colors(self.behaviour.colors());
        self.patterns.set_prompt(self.behaviour.prompt());
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

        let colors = self.patterns.colors();
        utils::clear_area(frame, area, colors.normal.bg);
        if area.top() > 0 && self.behaviour.has_header() {
            let header_area = Rect::new(area.x, area.y.saturating_sub(1), area.width, 1);
            let header_style = colors.header.unwrap_or_default();
            utils::clear_area(frame, header_area, header_style.bg);
            self.behaviour
                .draw_header(frame, header_area.inner(Margin::new(1, 0)), (&header_style).into());
        }

        self.patterns.draw(frame, area.inner(Margin::new(1, 0)));
    }

    /// Highlights picker item by name.
    pub fn highlight_item(&mut self, name: &str) {
        self.patterns.items.list.highlight_item_by_name(name);
    }

    fn run_validation(&mut self) {
        let error_pos = self.behaviour.validate(self.patterns.value());
        self.patterns.set_error(error_pos);
    }

    fn remember_pattern(&mut self) {
        let pattern = self.patterns.value();
        self.current = pattern.to_owned();
        self.behaviour.add_item(pattern);
        self.save_history_file();
    }

    fn save_history_file(&mut self) {
        if let Some(worker) = &self.worker {
            worker.borrow_mut().save_history(self.app_data.borrow().history.clone());
        }
    }

    fn complete_with_selected_item(&mut self) {
        if let Some(pattern) = self.patterns.items.get_highlighted_item_name().map(String::from) {
            self.patterns.set_value(pattern);
            self.run_validation();
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
            if self.behaviour.can_remove(self.patterns.items.get_highlighted())
                && let Some(removed) = self.patterns.items.remove_highlighted()
                && self.behaviour.remove_item(&removed)
            {
                self.save_history_file();
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
            self.remember_pattern();
            self.is_visible = false;

            return self
                .behaviour
                .navigate_into(self.patterns.value(), self.patterns.get_highlighted_item_name());
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

            self.remember_pattern();
            self.is_visible = false;

            return self
                .behaviour
                .navigate_into(self.patterns.value(), self.patterns.get_highlighted_item_name());
        }

        let result = self.behaviour.process_event(event, &mut self.patterns, &self.app_data);
        if result != ResponseEvent::NotHandled {
            return result;
        }

        self.patterns.process_event(event);
        self.run_validation();

        ResponseEvent::Handled
    }
}
