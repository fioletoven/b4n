use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::rc::Rc;

use crate::ui::{colors::TextColors, theme::SelectColors, ResponseEvent, Responsive, Table};

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
        let filter = Input::new(Style::default().fg(colors.filter.fg).bg(colors.filter.bg), filter_show_cursor);

        Select {
            items: list,
            colors,
            filter,
            filter_auto_hide,
        }
    }

    /// Adds prompt to the [`Select`] instance.
    pub fn with_prompt(mut self, prompt: String) -> Self {
        self.set_prompt(prompt);
        self
    }

    /// Sets prompt for the filter input.
    pub fn set_prompt(&mut self, prompt: String) {
        self.filter.set_prompt(Some((
            prompt,
            Style::default().fg(self.colors.prompt.fg).bg(self.colors.prompt.bg),
        )));
    }

    /// Sets colors for the filter input and list lines.
    pub fn set_colors(&mut self, colors: SelectColors) {
        self.filter
            .set_style(Style::default().fg(colors.filter.fg).bg(colors.filter.bg));
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
            frame.render_widget(Paragraph::new(self.get_resource_names(list)), list_area);
        }

        if draw_filter {
            self.filter.draw(frame, layout[0]);
        }
    }

    /// Returns resource names as formatted rows.
    fn get_resource_names<'a>(&self, resources: Vec<(String, bool)>) -> Vec<Line<'a>> {
        let mut result = Vec::with_capacity(resources.len());

        for (name, is_active) in resources {
            let colors = if is_active {
                self.colors.normal_hl
            } else {
                self.colors.normal
            };

            result.push(Line::from(get_resource_row(name, colors)));
        }

        result
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

/// Dims part of the line text between `[` and `]`.  
/// It removes these characters from the output.
fn get_resource_row<'a>(line: String, colors: TextColors) -> Vec<Span<'a>> {
    if line.contains('[') {
        let mut result = Vec::with_capacity(5);
        result.push(Span::raw(" "));

        let split = line.splitn(2, '[').collect::<Vec<&str>>();
        result.push(Span::styled(split[0].to_owned(), Style::new().fg(colors.fg).bg(colors.bg)));

        let split = split[1].rsplitn(2, ']').collect::<Vec<&str>>();
        if split.len() == 2 {
            result.push(Span::styled(
                split[1].to_owned(),
                Style::new().fg(colors.fg).bg(colors.bg).dim(),
            ));
        }
        result.push(Span::styled(split[0].to_owned(), Style::new().fg(colors.fg).bg(colors.bg)));

        result.push(Span::raw("\n"));
        result
    } else {
        vec![
            Span::raw(" "),
            Span::styled(line, Style::new().fg(colors.fg).bg(colors.bg)),
            Span::raw("\n"),
        ]
    }
}
