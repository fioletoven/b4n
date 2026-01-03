use b4n_config::themes::{TextColors, Theme};
use crossterm::event::KeyModifiers;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Position, Rect};
use ratatui_core::terminal::Frame;
use ratatui_core::widgets::Widget;

use crate::MouseEventKind;
use crate::table::ViewType;
use crate::{ResponseEvent, Responsive, TuiEvent, table::Table};

const MAX_ITEMS_ON_SCREEN: u16 = 25;

/// List widget for TUI.
#[derive(Default)]
pub struct List<T: Table> {
    pub items: T,
    pub area: Rect,
}

impl<T: Table> List<T> {
    /// Creates new [`List`] instance.
    pub fn new(list: T) -> Self {
        List {
            items: list,
            area: Rect::default(),
        }
    }

    /// Returns height needed to display items on screen.
    pub fn get_screen_height(&self) -> u16 {
        u16::try_from(self.items.len())
            .unwrap_or(MAX_ITEMS_ON_SCREEN)
            .min(MAX_ITEMS_ON_SCREEN)
    }

    /// Returns `true` if anything on the list is highlighted.
    pub fn is_anything_highlighted(&self) -> bool {
        self.items.get_highlighted_item_name().is_some()
    }

    /// Highlights first item.
    pub fn highlight_first(&mut self) {
        self.items.filter(None);
        self.items.highlight_first_item();
    }

    /// Highlights an item by name.
    pub fn highlight(&mut self, name: &str) {
        self.items.filter(None);
        self.items.highlight_item_by_name(name);
    }

    /// Highlights an item by uid.
    pub fn highlight_by_uid(&mut self, uid: &str) {
        self.items.filter(None);
        self.items.highlight_item_by_uid(uid);
    }

    /// Draws [`List`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        self.area = area;
        self.items.update_page(self.area.height);
        if let Some(list) = self
            .items
            .get_paged_items(theme, ViewType::Full, usize::from(self.area.width))
        {
            frame.render_widget(&mut ListWidget { list }, self.area);
        }
    }
}

impl<T: Table> Responsive for List<T> {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        match event {
            TuiEvent::Key(key) => {
                if key.modifiers == KeyModifiers::ALT {
                    return ResponseEvent::Handled;
                }

                self.items.process_event(event);
            },
            TuiEvent::Mouse(mouse) => {
                if mouse.kind == MouseEventKind::LeftClick && self.area.contains(Position::new(mouse.column, mouse.row)) {
                    let line = mouse.row.saturating_sub(self.area.y);
                    self.items.highlight_item_by_line(line);
                }

                self.items.process_event(event);
            },
            TuiEvent::Command(_) => (),
        }

        ResponseEvent::Handled
    }
}

/// Widget that renders all visible rows in a list.\
/// **Note** that it removes `[` and `]` characters from the output dimming the inside text.
pub struct ListWidget {
    pub list: Vec<(String, TextColors)>,
}

impl Widget for &mut ListWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let x = area.left();
        let y = area.top();
        for (i, row) in self.list.iter().enumerate() {
            let mut is_dimmed = false;
            let mut skipped = 0;
            for (j, char) in row.0.chars().enumerate() {
                if !is_dimmed && char == '[' {
                    is_dimmed = true;
                    skipped += 1;
                    continue;
                } else if is_dimmed && char == ']' {
                    is_dimmed = false;
                    skipped += 1;
                    continue;
                }

                let colors = row.1;
                let buf = &mut buf[(x + j as u16 - skipped, y + i as u16)];
                if is_dimmed {
                    buf.set_char(char).set_fg(colors.dim).set_bg(colors.bg);
                } else {
                    buf.set_char(char).set_style(&colors);
                }
            }
        }
    }
}
