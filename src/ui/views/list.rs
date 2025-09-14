use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Paragraph, Widget},
};

use crate::{
    core::{SharedAppData, SharedAppDataExt},
    ui::{KeyCommand, ResponseEvent, Responsive, Table, TuiEvent, ViewType, colors::TextColors},
};

/// List viewer.
pub struct ListViewer<T: Table> {
    pub table: T,
    pub view: ViewType,
    app_data: SharedAppData,
}

impl<T: Table> ListViewer<T> {
    /// Creates new [`ListViewer`] instance.
    pub fn new(app_data: SharedAppData, list: T, view: ViewType) -> Self {
        ListViewer {
            table: list,
            view,
            app_data,
        }
    }

    /// Draws [`ListViewer`] on the provided frame area.\
    /// It draws only the visible elements respecting the height of the `area`.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        let theme = &self.app_data.borrow().theme;
        frame.render_widget(Block::new().style(&theme.colors.text), area);
        let area = layout[1].inner(Margin::new(1, 0));

        {
            let sort_symbols = self.table.get_sort_symbols();
            let mut header = HeaderWidget {
                header: self.table.get_header(self.view, usize::from(area.width)),
                colors: &theme.colors.header.text,
                background: theme.colors.text.bg,
                view: self.view,
                sort_symbols: &sort_symbols,
            };
            frame.render_widget(&mut header, layout[0]);
        }

        self.table.update_page(area.height);
        if let Some(list) = self.table.get_paged_items(theme, self.view, usize::from(area.width)) {
            frame.render_widget(Paragraph::new(get_items(&list)).style(&theme.colors.text), area);
        }
    }
}

/// Returns formatted items rows.
fn get_items(items: &Vec<(String, TextColors)>) -> Vec<Line<'_>> {
    let mut result = Vec::with_capacity(items.len());

    for (text, colors) in items {
        result.push(Line::styled(text, colors));
    }

    result
}

impl<T: Table> Responsive for ListViewer<T> {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let TuiEvent::Key(key) = event
            && key.code == KeyCode::Char('0')
            && key.modifiers == KeyModifiers::ALT
            && self.view != ViewType::Full
        {
            return ResponseEvent::Handled;
        }

        if self.table.process_event(event) == ResponseEvent::Handled {
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateSelect) {
            self.table.select_highlighted_item();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateInvertSelection) {
            self.table.invert_selection();
            return ResponseEvent::Handled;
        }

        ResponseEvent::NotHandled
    }
}

/// Widget that renders header for the items list pane.\
/// It underlines sort symbol inside each column name.
struct HeaderWidget<'a> {
    pub header: &'a str,
    pub colors: &'a TextColors,
    pub background: Color,
    pub view: ViewType,
    pub sort_symbols: &'a [char],
}

impl Widget for &mut HeaderWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let x = area.left() + 1;
        let y = area.top();
        let max_x = area.left() + buf.area.width - 1;

        buf[(x - 1, y)].set_char('').set_fg(self.colors.bg).set_bg(self.background);
        buf[(max_x, y)].set_char('').set_fg(self.colors.bg).set_bg(self.background);

        let mut column_no = if self.view == ViewType::Full { 0 } else { 1 };
        let mut in_column = false;
        let mut highlighted = false;

        for (i, char) in self.header.chars().enumerate() {
            let x = x + i as u16;
            if x >= max_x {
                break;
            }

            if char != ' ' && !in_column {
                in_column = true;
                highlighted = false;
            } else if char == ' ' && in_column {
                in_column = false;
                column_no += 1;
            }

            let can_be_highlighted = column_no < self.sort_symbols.len()
                && self.sort_symbols[column_no] != ' '
                && char == self.sort_symbols[column_no];

            if in_column && can_be_highlighted && !highlighted {
                highlighted = true;
                buf[(x, y)].set_style(Style::default().underlined());
            }

            if char == '↑' || char == '↓' {
                buf[(x, y)].set_char(char).set_fg(self.colors.dim).set_bg(self.colors.bg);
            } else {
                buf[(x, y)].set_char(char).set_fg(self.colors.fg).set_bg(self.colors.bg);
            }
        }
    }
}
