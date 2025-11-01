use b4n_config::{keys::KeyCombination, themes::SelectColors, themes::TextColors};
use b4n_tui::{ResponseEvent, Responsive, TuiEvent};
use crossterm::event::{KeyCode, KeyModifiers};
use delegate::delegate;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::Widget;
use std::rc::Rc;

use crate::ui::{Table, widgets::ErrorHighlightMode};

use super::Input;

/// Select widget for TUI.
#[derive(Default)]
pub struct Select<T: Table> {
    pub items: T,
    pub area: Rect,
    colors: SelectColors,
    filter: Input,
    filter_auto_hide: bool,
}

impl<T: Table> Select<T> {
    /// Creates new [`Select`] instance.
    /// * `filter_auto_hide` - hides filter input when no filter is present.
    /// * `filter_show_cursor` - indicates if filter input should show cursor.
    pub fn new(list: T, colors: SelectColors, filter_auto_hide: bool, filter_show_cursor: bool) -> Self {
        let filter = Input::new(colors.filter.input)
            .with_cursor(
                filter_show_cursor && colors.cursor.is_some(),
                colors.cursor.unwrap_or_default(),
            )
            .with_error_colors(colors.filter.error);

        Select {
            items: list,
            area: Rect::default(),
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

    /// Adds a set of characters that should be accented by the [`Select`] instance.
    pub fn with_accent_characters(mut self, highlight: impl Into<String>) -> Self {
        self.filter.set_accent_characters(Some(highlight.into()));
        self
    }

    /// Sets prompt for the filter input.
    pub fn set_prompt(&mut self, prompt: impl Into<String>) {
        self.filter
            .set_prompt(Some((prompt, self.colors.filter.prompt.unwrap_or_default())));
    }

    /// Sets colors for the filter input and list lines.
    pub fn set_colors(&mut self, colors: SelectColors) {
        self.filter.set_colors(colors.filter.input);
        self.filter.set_prompt_colors(colors.filter.prompt.unwrap_or_default());
        self.filter.set_error_colors(colors.filter.error);
        self.colors = colors;
    }

    delegate! {
        to self.filter {
            pub fn set_cursor(&mut self, show_cursor: bool);
            pub fn set_error_mode(&mut self, mode: ErrorHighlightMode);
            pub fn has_error(&self) -> bool;
            pub fn set_error(&mut self, error_index: Option<usize>);
            pub fn prompt(&self) -> Option<&str>;
            pub fn value(&self) -> &str;
        }
    }

    /// Sets the filter value.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.filter.set_value(value);
        self.update_items_filter();
    }

    /// Returns `true` if anything on the select list is highlighted.
    pub fn is_anything_highlighted(&self) -> bool {
        self.items.get_highlighted_item_name().is_some()
    }

    /// Resets the filter.
    pub fn reset(&mut self) {
        self.filter.reset();
        self.items.filter(None);
    }

    /// Highlights an item by name.
    pub fn highlight(&mut self, selected_name: &str) {
        self.items.filter(None);
        self.items.highlight_item_by_name(selected_name);
    }

    /// Highlights an item by uid.
    pub fn highlight_by_uid(&mut self, selected_uid: &str) {
        self.items.filter(None);
        self.items.highlight_item_by_uid(selected_uid);
    }

    /// Draws [`Select`] on the provided frame area.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let draw_filter = !self.filter_auto_hide || self.items.get_filter().is_some();
        let layout = get_layout(area, draw_filter);
        self.area = if draw_filter { layout[1] } else { layout[0] };
        self.items.update_page(self.area.height);
        if let Some(list) = self.items.get_paged_names(usize::from(self.area.width.max(2) - 2)) {
            frame.render_widget(
                &mut ListWidget {
                    list,
                    normal: &self.colors.normal,
                    highlighted: &self.colors.normal_hl,
                },
                self.area,
            );
        }

        if draw_filter {
            self.filter.draw(frame, layout[0]);
        }
    }

    /// Updates filter applied on items.
    pub fn update_items_filter(&mut self) {
        if self.filter.value().is_empty() {
            self.items.filter(None);
        } else if let Some(filter) = self.items.get_filter() {
            if self.filter.value() != filter {
                self.filter_and_highlight();
            }
        } else {
            self.filter_and_highlight();
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
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let key = match event {
            TuiEvent::Key(key) => key,
            _ => &KeyCombination::default(),
        };

        if key.modifiers == KeyModifiers::ALT {
            return ResponseEvent::Handled;
        }

        // Process Home and End keys directly by filter input if we show cursor
        // (that means move cursor to start or end of the filter input text).
        if (self.filter.is_cursor_visible() && (key.code == KeyCode::Home || key.code == KeyCode::End))
            || self.items.process_event(event) == ResponseEvent::NotHandled
        {
            self.filter.process_event(event);
            self.update_items_filter();
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

/// Widget that renders all visible rows in select.\
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
