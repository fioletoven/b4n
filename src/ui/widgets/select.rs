use crossterm::event::{KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Widget,
};
use std::rc::Rc;

use crate::ui::{ResponseEvent, Responsive, Table, colors::TextColors, theme::SelectColors};

use super::Input;

/// Select widget for TUI.
#[derive(Default)]
pub struct Select<T: Table> {
    pub items: T,
    colors: SelectColors,
    filter: Input,
    filter_auto_hide: bool,
}

impl<T: Table> Select<T> {
    /// Creates new [`Select`] instance.
    /// * `filter_auto_hide` - hides filter input when no filter is present.
    /// * `filter_show_cursor` - indicates if filter input should show cursor.
    pub fn new(list: T, colors: SelectColors, filter_auto_hide: bool, filter_show_cursor: bool) -> Self {
        let filter = Input::new(&colors.filter.input, filter_show_cursor);

        Select {
            items: list,
            colors,
            filter,
            filter_auto_hide,
        }
    }

    /// Adds prompt to the [`Select`] instance.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.set_prompt(prompt);
        self
    }

    /// Sets prompt for the filter input.
    pub fn set_prompt(&mut self, prompt: impl Into<String>) {
        self.filter.set_prompt(Some((prompt, &self.colors.filter.prompt)));
    }

    /// Sets colors for the filter input and list lines.
    pub fn set_colors(&mut self, colors: SelectColors) {
        self.filter.set_style(&colors.filter.input);
        self.colors = colors;
    }

    /// Sets whether to show the cursor in the filter input.
    pub fn set_cursor(&mut self, show_cursor: bool) {
        self.filter.set_cursor(show_cursor);
    }

    /// Resets filter.
    pub fn reset(&mut self) {
        self.filter.reset();
        self.items.filter(None);
    }

    /// Highlights an item by name and group.
    pub fn select(&mut self, selected_name: &str, selected_group: &str) {
        self.items.filter(None);
        if selected_group.is_empty()
            || !self
                .items
                .highlight_item_by_name(&format!("{}.{}", selected_name, selected_group))
        {
            self.items.highlight_item_by_name(selected_name);
        }
    }

    /// Draws [`Select`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let draw_filter = !self.filter_auto_hide || self.items.get_filter().is_some();
        let layout = get_layout(area, draw_filter);
        let list_area = if draw_filter { layout[1] } else { layout[0] };
        self.items.update_page(list_area.height);
        if let Some(list) = self.items.get_paged_names(usize::from(list_area.width.max(2) - 2)) {
            frame.render_widget(
                &mut ListWidget {
                    list,
                    normal: &self.colors.normal,
                    highlighted: &self.colors.normal_hl,
                },
                list_area,
            );
        }

        if draw_filter {
            self.filter.draw(frame, layout[0]);
        }
    }

    fn filter_and_highlight(&mut self) {
        self.items.filter(Some(self.filter.value().to_owned()));
        self.items.highlight_item_by_name_start(self.filter.value());
        if self.items.get_highlighted_item_index().is_none() {
            self.items.highlight_first_item();
        }
    }
}

impl<T: Table> Responsive for Select<T> {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if key.modifiers == KeyModifiers::ALT {
            return ResponseEvent::Handled;
        }

        if self.items.process_key(key) == ResponseEvent::NotHandled {
            self.filter.process_key(key);
            if self.filter.value().is_empty() {
                self.items.filter(None);
            } else {
                if let Some(filter) = self.items.get_filter() {
                    if self.filter.value() != filter {
                        self.filter_and_highlight();
                    }
                } else {
                    self.filter_and_highlight();
                }
            }
        }

        ResponseEvent::Handled
    }
}

fn get_layout(area: Rect, is_filter_shown: bool) -> Rc<[Rect]> {
    let constraints = if is_filter_shown {
        vec![Constraint::Length(1), Constraint::Fill(1)]
    } else {
        vec![Constraint::Fill(1)]
    };

    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
}

/// Widget that renders all visible rows in select.  
/// **Note** that it removes `[` and `]` characters from the output dimming the inside text.
struct ListWidget<'a> {
    pub list: Vec<(String, bool)>,
    pub normal: &'a TextColors,
    pub highlighted: &'a TextColors,
}

impl Widget for &mut ListWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let x = area.left() + 1;
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

                let colors = if row.1 { self.highlighted } else { self.normal };
                let buf = &mut buf[(x + j as u16 - skipped, y + i as u16)];
                if is_dimmed {
                    buf.set_char(char).set_fg(colors.dim).set_bg(colors.bg);
                } else {
                    buf.set_char(char).set_style(colors);
                }
            }
        }
    }
}
