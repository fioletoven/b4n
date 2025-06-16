use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    core::SharedAppData,
    ui::{
        ResponseEvent, Responsive, Table,
        theme::SelectColors,
        utils::center_horizontal,
        widgets::{InputValidator, Select, ValidatorKind},
    },
};

use super::ActionsList;

const DEFAULT_PROMPT: &str = " ";

/// Command Palette widget for TUI.
#[derive(Default)]
pub struct CommandPalette {
    pub is_visible: bool,
    app_data: SharedAppData,
    steps: Vec<Step>,
    index: usize,
    width: u16,
    response: Option<Box<dyn Fn(Vec<String>) -> ResponseEvent>>,
}

impl CommandPalette {
    /// Creates new [`CommandPalette`] instance.
    pub fn new(app_data: SharedAppData, actions: ActionsList, width: u16) -> Self {
        let colors = app_data.borrow().theme.colors.command_palette.clone();
        Self {
            is_visible: false,
            app_data,
            steps: vec![Step::new(actions, colors)],
            index: 0,
            width,
            response: None,
        }
    }

    /// Adds additional actions step to the command palette.
    pub fn new_actions_step(mut self, actions: ActionsList) -> Self {
        let colors = self.app_data.borrow().theme.colors.command_palette.clone();
        self.steps.push(Step::new(actions, colors));
        self
    }

    /// Adds additional input step to the command palette.
    pub fn new_input_step(mut self, initial_value: impl Into<String>) -> Self {
        let colors = self.app_data.borrow().theme.colors.command_palette.clone();
        let mut step = Step::new(ActionsList::default(), colors);
        step.select.set_value(initial_value);
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
        self.steps[index].select.highlight(name, "");
        self
    }

    /// Sets closure that will be executed to generate [`ResponseEvent`] when all steps will be processed.
    pub fn with_response<F>(mut self, response: F) -> Self
    where
        F: Fn(Vec<String>) -> ResponseEvent + 'static,
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
        let area = center_horizontal(area, width, self.select().items.list.len() + 1);

        self.clear_area(frame, area);
        self.select_mut().draw(frame, area);
    }

    fn clear_area(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let colors = &self.app_data.borrow().theme.colors;
        let block = Block::new().style(Style::default().bg(colors.command_palette.normal.bg));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }

    #[inline]
    fn select(&self) -> &Select<ActionsList> {
        &self.steps[self.index].select
    }

    #[inline]
    fn select_mut(&mut self) -> &mut Select<ActionsList> {
        &mut self.steps[self.index].select
    }

    fn next_step(&mut self, force_selected_value: bool) -> bool {
        if self.index + 1 >= self.steps.len() {
            return false;
        }

        if self.select().is_anything_highlighted() && (self.select().value().is_empty() || force_selected_value) {
            let value = self.select().items.get_highlighted_item_name().unwrap_or_default().to_owned();
            self.select_mut().set_value(value);
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

    fn can_advance_to_next_step(&self) -> bool {
        !self.select().has_error()
            && (self.select().is_anything_highlighted() || (self.select().items.len() == 0 && !self.select().value().is_empty()))
    }
}

impl Responsive for CommandPalette {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if key.code == KeyCode::Esc {
            if self.index > 0 {
                self.index -= 1;
            } else {
                self.is_visible = false;
            }

            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Tab && self.steps.len() > 1 && self.can_advance_to_next_step() {
            self.next_step(false);
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Enter {
            if self.steps.len() == 1
                || (self.select().value().is_empty() && !self.select().is_anything_highlighted())
                || (!self.select().has_error() && !self.next_step(true))
            {
                self.is_visible = false;
                if let Some(response) = &self.response {
                    return (response)(self.build_response());
                }

                if let Some(index) = self.select().items.list.get_highlighted_item_index() {
                    if let Some(items) = &self.select().items.list.items {
                        return items[index].data.response.clone();
                    }
                }
            }

            return ResponseEvent::Handled;
        }

        let response = self.select_mut().process_key(key);
        self.steps[self.index].validate();

        response
    }
}

/// Step for the Command Palette.
struct Step {
    select: Select<ActionsList>,
    prompt: Option<String>,
    validator: InputValidator,
}

impl Step {
    /// Creates new [`Step`] instance.
    fn new(list: ActionsList, colors: SelectColors) -> Self {
        Self {
            select: Select::new(list, colors, false, true).with_prompt(DEFAULT_PROMPT),
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
