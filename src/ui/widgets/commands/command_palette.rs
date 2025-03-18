use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    app::SharedAppData,
    ui::{ResponseEvent, Responsive, utils::center_horizontal, widgets::Select},
};

use super::ActionsList;

/// Command Palette widget for TUI.
#[derive(Default)]
pub struct CommandPalette {
    pub is_visible: bool,
    app_data: SharedAppData,
    actions: Select<ActionsList>,
    width: u16,
}

impl CommandPalette {
    /// Creates new [`CommandPalette`] instance.
    pub fn new(app_data: SharedAppData, actions: ActionsList, width: u16) -> Self {
        let colors = app_data.borrow().theme.colors.command_palette.clone();

        Self {
            is_visible: false,
            app_data,
            actions: Select::new(actions, colors, false, true).with_prompt(" "),
            width,
        }
    }

    /// Sets command palette prompt.
    pub fn set_prompt(&mut self, prompt: &str) {
        self.actions.set_prompt(format!("{} ", prompt));
    }

    /// Selects one of the command palette actions by its name.
    pub fn select(&mut self, name: &str) {
        self.actions.select(name, "");
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

        let colors = &self.app_data.borrow().theme.colors;
        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let area = center_horizontal(area, width, self.actions.items.list.len() + 1);
        let block = Block::new().style(Style::default().bg(colors.command_palette.normal.bg));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        self.actions.draw(frame, area);
    }
}

impl Responsive for CommandPalette {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if key.code == KeyCode::Esc {
            self.is_visible = false;
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Enter {
            self.is_visible = false;
            if let Some(index) = self.actions.items.list.get_highlighted_item_index() {
                if let Some(items) = &self.actions.items.list.items {
                    return items[index].data.response.clone();
                }
            }

            return ResponseEvent::Handled;
        }

        self.actions.process_key(key)
    }
}
