use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::{
    app::SharedAppData,
    ui::{ResponseEvent, Responsive, Table, ViewType, colors::TextColors},
};

/// Resources list pane.
pub struct ListPane<T: Table> {
    pub items: T,
    pub view: ViewType,
    app_data: SharedAppData,
}

impl<T: Table> ListPane<T> {
    /// Creates new resource list pane.
    pub fn new(app_data: SharedAppData, list: T, view: ViewType) -> Self {
        ListPane {
            items: list,
            view,
            app_data,
        }
    }

    /// Draws [`ListPane`] on the provided frame area.  
    /// It draws only the visible elements respecting the height of the `area`.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        let list_width = if area.width > 2 { usize::from(area.width) - 2 } else { 2 };

        {
            let sort_symbols = self.items.get_sort_symbols();
            let mut header = HeaderWidget {
                header: self.items.get_header(self.view, list_width),
                colors: &self.app_data.borrow().config.theme.colors.header.text,
                view: self.view,
                sort_symbols: &sort_symbols,
            };
            frame.render_widget(&mut header, layout[0]);
        }

        self.items.update_page(layout[1].height);
        if let Some(list) = self
            .items
            .get_paged_items(&self.app_data.borrow().config.theme, self.view, list_width)
        {
            frame.render_widget(Paragraph::new(self.get_resources(list)), layout[1]);
        }
    }

    /// Returns formatted resources rows.
    fn get_resources(&self, resources: Vec<(String, TextColors)>) -> Vec<Line> {
        let mut result = Vec::with_capacity(resources.len());

        for (text, colors) in resources {
            let row = Span::styled(text, &colors);
            result.push(Line::from(vec![Span::raw(" "), row, Span::raw("\n")]));
        }

        result
    }
}

impl<T: Table> Responsive for ListPane<T> {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if self.items.process_key(key) == ResponseEvent::Handled {
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char(' ') {
            if key.modifiers == KeyModifiers::CONTROL {
                self.items.invert_selection();
            } else {
                self.items.select_highlighted_item();
            }

            return ResponseEvent::Handled;
        }

        ResponseEvent::NotHandled
    }
}

/// Widget that renders header for the resources list pane.  
/// It underlines sort symbol inside each column name.
struct HeaderWidget<'a> {
    pub header: &'a str,
    pub colors: &'a TextColors,
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
        let width = buf.area.width - 1;

        buf[Position::new(x - 1, y)]
            .set_char('')
            .set_fg(self.colors.bg)
            .set_bg(Color::Reset);
        buf[Position::new(width, y)]
            .set_char('')
            .set_fg(self.colors.bg)
            .set_bg(Color::Reset);

        let mut column_no = if self.view == ViewType::Full { 0 } else { 1 };
        let mut in_column = false;
        let mut highlighted = false;

        for (i, char) in self.header.chars().enumerate() {
            let x = x + i as u16;
            if x >= width {
                break;
            }

            if char != ' ' && !in_column {
                in_column = true;
                highlighted = false;
            } else if char == ' ' && in_column {
                in_column = false;
                column_no += 1;
            }

            if in_column && !highlighted && column_no < self.sort_symbols.len() {
                if self.sort_symbols[column_no] != ' ' && char == self.sort_symbols[column_no] {
                    highlighted = true;
                    buf[Position::new(x, y)].set_style(Style::default().underlined());
                }
            }

            if char == '↑' || char == '↓' {
                buf[Position::new(x, y)]
                    .set_char(char)
                    .set_fg(self.colors.dim)
                    .set_bg(self.colors.bg);
            } else {
                buf[Position::new(x, y)]
                    .set_char(char)
                    .set_fg(self.colors.fg)
                    .set_bg(self.colors.bg);
            }
        }
    }
}
