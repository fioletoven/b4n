use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    core::SharedAppData,
    ui::{ResponseEvent, Responsive, Table, utils::center_horizontal, widgets::Select},
};

use super::ActionsList;

const DEFAULT_PROMPT: &str = " ";

/// Command Palette widget for TUI.
#[derive(Default)]
pub struct CommandPalette {
    pub is_visible: bool,
    app_data: SharedAppData,
    steps: Vec<Select<ActionsList>>,
    prompts: Vec<Option<String>>,
    index: usize,
    width: u16,
}

impl CommandPalette {
    /// Creates new [`CommandPalette`] instance.
    pub fn new(app_data: SharedAppData, actions: ActionsList, width: u16) -> Self {
        let colors = app_data.borrow().theme.colors.command_palette.clone();

        Self {
            is_visible: false,
            app_data,
            steps: vec![Select::new(actions, colors, false, true).with_prompt(DEFAULT_PROMPT)],
            prompts: vec![None],
            index: 0,
            width,
        }
    }

    /// Adds additional actions step to the command palette.
    pub fn with_actions_step(mut self, actions: ActionsList) -> Self {
        let colors = self.app_data.borrow().theme.colors.command_palette.clone();
        self.steps
            .push(Select::new(actions, colors, false, true).with_prompt(DEFAULT_PROMPT));
        self.prompts.push(None);
        self
    }

    /// Adds additional input step to the command palette.
    pub fn with_input_step(mut self, initial_value: impl Into<String>) -> Self {
        let colors = self.app_data.borrow().theme.colors.command_palette.clone();
        let mut select = Select::new(ActionsList::default(), colors, false, true).with_prompt(DEFAULT_PROMPT);
        select.set_value(initial_value);
        self.steps.push(select);
        self.prompts.push(None);
        self
    }

    /// Sets prompt for the last added step of the command palette.
    pub fn with_prompt(mut self, prompt: &str) -> Self {
        let index = self.steps.len().saturating_sub(1);
        self.steps[index].set_prompt(format!("{prompt}{DEFAULT_PROMPT}"));
        self.prompts[index] = Some(format!("{prompt}{DEFAULT_PROMPT}"));
        self
    }

    /// Selects one of the actions from the last added step of the command palette.
    pub fn with_selected(mut self, name: &str) -> Self {
        let index = self.steps.len().saturating_sub(1);
        self.steps[index].select(name, "");
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
        &self.steps[self.index]
    }

    #[inline]
    fn select_mut(&mut self) -> &mut Select<ActionsList> {
        &mut self.steps[self.index]
    }

    fn next_step(&mut self) -> bool {
        if self.index + 1 >= self.steps.len() {
            return false;
        }

        if self.select().is_anything_highlighted() && self.select().value().is_empty() {
            let value = self.select().items.get_highlighted_item_name().unwrap_or_default().to_owned();
            self.select_mut().set_value(value);
        }

        if self.steps[self.index + 1].value().is_empty() {
            let value = self.select().value().to_owned();
            self.steps[self.index + 1].set_value(value);
        }

        let prompt = format!(
            "{0}{1}{DEFAULT_PROMPT}{2}",
            self.build_prev_prompt(),
            self.select().value(),
            self.prompts[self.index + 1].as_deref().unwrap_or(DEFAULT_PROMPT)
        );

        self.index += 1;
        self.select_mut().set_prompt(prompt);

        true
    }

    fn build_prev_prompt(&self) -> String {
        let mut result = String::new();
        for i in 0..self.index {
            result.push_str(self.steps[i].value());
            result.push('');
            result.push(' ');
        }

        result
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

        if key.code == KeyCode::Tab
            && (self.select().is_anything_highlighted() || (self.select().items.len() == 0 && !self.select().value().is_empty()))
        {
            if !self.next_step() {
                self.index = 0;
            }

            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Enter {
            if self.steps.len() == 1
                || (self.select().value().is_empty() && !self.select().is_anything_highlighted())
                || !self.next_step()
            {
                self.is_visible = false;
                if let Some(index) = self.select().items.list.get_highlighted_item_index() {
                    if let Some(items) = &self.select().items.list.items {
                        return items[index].data.response.clone();
                    }
                }
            }

            return ResponseEvent::Handled;
        }

        self.select_mut().process_key(key)
    }
}
