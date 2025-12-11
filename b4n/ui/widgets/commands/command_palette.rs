use b4n_config::keys::KeyCommand;
use b4n_config::themes::SelectColors;
use b4n_tui::widgets::{ErrorHighlightMode, InputValidator, ValidatorKind};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table, utils::center_horizontal};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::ui::widgets::{ActionsList, Select};

const DEFAULT_PROMPT: &str = " ";

/// Command Palette widget for TUI.
#[derive(Default)]
pub struct CommandPalette {
    pub is_visible: bool,
    app_data: SharedAppData,
    header: Option<String>,
    steps: Vec<Step>,
    index: usize,
    width: u16,
    response: Option<Box<dyn FnOnce(Vec<String>) -> ResponseEvent>>,
}

impl CommandPalette {
    /// Creates new [`CommandPalette`] instance.
    pub fn new(app_data: SharedAppData, actions: ActionsList, width: u16) -> Self {
        let colors = app_data.borrow().theme.colors.command_palette.clone();
        Self {
            app_data,
            steps: vec![Step::new(actions, colors)],
            width,
            ..Default::default()
        }
    }

    /// Adds header to the command palette.
    pub fn with_header(mut self, text: impl Into<String>) -> Self {
        self.header = Some(text.into());
        self
    }

    /// Adds additional actions step to the command palette.
    pub fn with_step(mut self, mut step: Step) -> Self {
        let colors = self.app_data.borrow().theme.colors.command_palette.clone();
        step.select.set_colors(colors);
        self.steps.push(step);
        self
    }

    /// Sets validator for the last added step of the command palette.
    pub fn with_validator(mut self, validator: ValidatorKind) -> Self {
        let index = self.steps.len().saturating_sub(1);
        self.steps[index].validator = InputValidator::new(validator);
        self
    }

    /// Sets prompt for the last added step of the command palette.
    pub fn with_prompt(mut self, prompt: &str) -> Self {
        let index = self.steps.len().saturating_sub(1);
        self.steps[index].select.set_prompt(format!("{prompt}{DEFAULT_PROMPT}"));
        self.steps[index].prompt = Some(format!("{prompt}{DEFAULT_PROMPT}"));
        self
    }

    /// Selects one of the actions from the last added step of the command palette.
    pub fn with_selected(mut self, name: &str) -> Self {
        let index = self.steps.len().saturating_sub(1);
        self.steps[index].select.highlight(name);
        self
    }

    /// Selects first action from the last added step of the command palette.
    pub fn with_first_selected(mut self) -> Self {
        let index = self.steps.len().saturating_sub(1);
        self.steps[index].select.highlight_first();
        self
    }

    /// Sets if text input is enabled for the last added step of the command palette.
    pub fn with_input(mut self, enabled: bool) -> Self {
        let index = self.steps.len().saturating_sub(1);
        self.steps[index].select.set_filter(enabled);
        self
    }

    /// Sets closure that will be executed to generate [`ResponseEvent`] when all steps will be processed.
    pub fn with_response<F>(mut self, response: F) -> Self
    where
        F: FnOnce(Vec<String>) -> ResponseEvent + 'static,
    {
        self.response = Some(Box::new(response));
        self
    }

    /// Marks [`CommandPalette`] as visible.
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Marks [`CommandPalette`] as hidden.
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Draws [`CommandPalette`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let area = center_horizontal(area, width, self.select().get_full_height());

        {
            let colors = &self.app_data.borrow().theme.colors;
            Self::clear_area(frame, area, colors.command_palette.normal.bg);
            if area.top() > 0
                && let Some(header) = self.header.as_deref()
            {
                let area = Rect::new(area.x, area.y.saturating_sub(1), area.width, 1);
                Self::clear_area(frame, area, colors.command_palette.header.unwrap_or_default().bg);
                self.draw_header(frame, area, header);
            }
        }

        self.select_mut().draw(frame, area);
    }

    fn clear_area(frame: &mut ratatui::Frame<'_>, area: Rect, color: Color) {
        let block = Block::new().style(Style::default().bg(color));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }

    fn draw_header(&self, frame: &mut ratatui::Frame<'_>, area: Rect, text: &str) {
        let colors = &self.app_data.borrow().theme.colors;
        let area = area.inner(Margin::new(1, 0));
        frame.render_widget(
            Paragraph::new(text).style(&colors.command_palette.header.unwrap_or_default()),
            area,
        );
    }

    fn select(&self) -> &Select<ActionsList> {
        &self.steps[self.index].select
    }

    fn select_mut(&mut self) -> &mut Select<ActionsList> {
        &mut self.steps[self.index].select
    }

    fn insert_highlighted_value(&mut self, overwrite_if_not_empty: bool) {
        if self.select().is_anything_highlighted() && (self.select().value().is_empty() || overwrite_if_not_empty) {
            let value = self.select().items.get_highlighted_item_name().unwrap_or_default().to_owned();
            self.select_mut().set_value(value);
        }
    }

    fn can_advance_to_next_step(&self) -> bool {
        !self.select().has_error()
            && self.index + 1 < self.steps.len()
            && (self.select().is_anything_highlighted() || (self.select().items.len() == 0 && !self.select().value().is_empty()))
    }

    fn next_step(&mut self) -> bool {
        if !self.can_advance_to_next_step() {
            return false;
        }

        if self.steps[self.index + 1].select.value().is_empty() {
            let value = self.select().value().to_owned();
            self.steps[self.index + 1].select.set_value(value);
        }

        let prompt = format!(
            "{0}{1}{DEFAULT_PROMPT}{2}",
            self.build_prev_prompt(),
            self.select().value(),
            self.steps[self.index + 1].prompt.as_deref().unwrap_or(DEFAULT_PROMPT)
        );

        self.index += 1;
        self.select_mut().set_prompt(prompt);

        true
    }

    fn build_prev_prompt(&self) -> String {
        let mut result = String::new();
        for i in 0..self.index {
            result.push_str(self.steps[i].select.value());
            result.push('');
            result.push(' ');
        }

        result
    }

    fn build_response(&self) -> Vec<String> {
        self.steps.iter().map(|s| s.select.value().to_owned()).collect()
    }

    fn process_enter_key(&mut self) -> ResponseEvent {
        self.insert_highlighted_value(false);

        if !self.select().has_error() && !self.select().value().is_empty() && (self.steps.len() == 1 || !self.next_step()) {
            self.is_visible = false;

            if self.steps.len() == self.index + 1
                && let Some(response) = self.response.take()
            {
                return (response)(self.build_response());
            }

            if let Some(index) = self.select().items.list.get_highlighted_item_index()
                && let Some(items) = &self.select().items.list.items
            {
                return items[index].data.response.clone();
            }
        }

        ResponseEvent::Handled
    }
}

impl Responsive for CommandPalette {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.app_data.has_binding(event, KeyCommand::CommandPaletteReset) {
            if self.index > 0 {
                self.index -= 1;
                return ResponseEvent::Handled;
            } else if !self.select().value().is_empty() {
                self.select_mut().reset();
                return ResponseEvent::Handled;
            }
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack)
            || event.is_out(MouseEventKind::LeftClick, self.select().area)
            || event.is_mouse(MouseEventKind::RightClick)
        {
            self.is_visible = false;
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateComplete) {
            self.insert_highlighted_value(true);
            return ResponseEvent::Handled;
        }

        if let Some(line) = event.get_clicked_line_no(MouseEventKind::LeftClick, KeyModifiers::NONE, self.select().area) {
            self.select_mut().items.highlight_item_by_line(line);
            return self.process_enter_key();
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateInto) {
            return self.process_enter_key();
        }

        let response = self.select_mut().process_event(event);
        self.steps[self.index].validate();

        response
    }
}

/// Builder for the command palette [`Step`].
pub struct StepBuilder {
    actions: Option<ActionsList>,
    initial_value: Option<String>,
    prompt: Option<String>,
    validator: InputValidator,
    colors: SelectColors,
}

impl StepBuilder {
    /// Creates new input [`Step`] builder.
    pub fn input(initial_value: impl Into<String>) -> Self {
        Self {
            actions: None,
            initial_value: Some(initial_value.into()),
            prompt: None,
            validator: InputValidator::new(ValidatorKind::None),
            colors: SelectColors::default(),
        }
    }

    /// Creates new actions [`Step`] builder.
    pub fn actions(actions: ActionsList) -> Self {
        Self {
            actions: Some(actions),
            initial_value: None,
            prompt: None,
            validator: InputValidator::new(ValidatorKind::None),
            colors: SelectColors::default(),
        }
    }

    /// Adds validator to the [`Step`].
    pub fn with_validator(mut self, validator: ValidatorKind) -> Self {
        self.validator = InputValidator::new(validator);
        self
    }

    /// Adds custom prompt to the [`Step`].
    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.prompt = Some(format!("{prompt}{DEFAULT_PROMPT}"));
        self
    }

    /// Adds custom select colors to the step.
    pub fn with_colors(mut self, colors: SelectColors) -> Self {
        self.colors = colors;
        self
    }

    /// Builds [`Step`] instance.
    pub fn build(self) -> Step {
        let list = self.actions.unwrap_or_default();
        let mut select = Select::new(list, self.colors, false, true).with_prompt(DEFAULT_PROMPT);
        select.set_error_mode(ErrorHighlightMode::Value);
        if let Some(initial_value) = self.initial_value {
            select.set_value(initial_value);
        }

        Step {
            select,
            prompt: self.prompt,
            validator: self.validator,
        }
    }
}

/// Step for the Command Palette.
pub struct Step {
    select: Select<ActionsList>,
    prompt: Option<String>,
    validator: InputValidator,
}

impl Step {
    /// Creates new [`Step`] instance.
    fn new(list: ActionsList, colors: SelectColors) -> Self {
        let mut select = Select::new(list, colors, false, true).with_prompt(DEFAULT_PROMPT);
        select.set_error_mode(ErrorHighlightMode::Value);

        Self {
            select,
            prompt: None,
            validator: InputValidator::new(ValidatorKind::None),
        }
    }

    /// Validates the current step using associated validator.
    fn validate(&mut self) -> bool {
        if let Err(error_index) = self.validator.validate(self.select.value()) {
            self.select.set_error(Some(error_index));
            false
        } else {
            self.select.set_error(None);
            true
        }
    }
}
