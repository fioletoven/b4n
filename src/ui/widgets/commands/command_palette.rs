use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::Style,
    widgets::{Block, Clear},
};

use crate::{
    app::{SharedAppData, lists::ActionsList},
    ui::{ResponseEvent, Responsive, widgets::Select},
};

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
        let colors = app_data.borrow().config.theme.colors.command_palette.clone();

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

    /// Marks [`CommandPalette`] as a visible.
    pub fn show(&mut self) {
        self.is_visible = true;
    }

    /// Marks [`CommandPalette`] as a hidden.
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Draws [`CommandPalette`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        if !self.is_visible {
            return;
        }

        let colors = &self.app_data.borrow().config.theme.colors;
        let width = std::cmp::min(area.width, self.width).max(2) - 2;
        let area = center(area, width, self.actions.items.list.len() + 1);
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

/// Centers horizontally a [`Rect`] within another [`Rect`] using the provided width and max height.
pub fn center(area: Rect, width: u16, max_height: usize) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center).areas(area);
    let top = if area.height > 2 { (area.height - 2).min(3) } else { 0 };
    let mut bottom = if area.height > 5 { (area.height - 5).min(6) } else { 0 };
    if area.height >= 7 && area.height <= 14 {
        bottom = area.height.saturating_sub(9).max(2);
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(top), Constraint::Fill(1), Constraint::Length(bottom)])
        .split(area);

    if usize::from(layout[1].height) > max_height {
        Rect::new(layout[1].x, layout[1].y, layout[1].width, max_height as u16)
    } else {
        layout[1]
    }
}
