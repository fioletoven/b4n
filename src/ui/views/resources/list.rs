use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    app::SharedAppData,
    ui::{colors::TextColors, ResponseEvent, Responsive, Table, ViewType},
};

/// Resources list pane
pub struct ListPane<T: Table> {
    pub items: T,
    pub view: ViewType,
    app_data: SharedAppData,
}

impl<T: Table> ListPane<T> {
    /// Creates new resource list pane
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

        let header = self.get_header(list_width);
        frame.render_widget(Paragraph::new(header), layout[0]);

        self.items.update_page(layout[1].height);
        if let Some(list) = self
            .items
            .get_paged_items(&self.app_data.borrow().config.theme, self.view, list_width)
        {
            frame.render_widget(Paragraph::new(self.get_resources(list)), layout[1]);
        }
    }

    /// Returns formatted header for resources rows
    fn get_header(&self, width: usize) -> Line {
        let header = self.items.get_header(self.view, width);
        let colors = &self.app_data.borrow().config.theme.colors;

        Line::from(vec![
            Span::styled("", Style::new().fg(colors.header.text.bg)),
            Span::styled(header, &colors.header.text),
            Span::styled("", Style::new().fg(colors.header.text.bg)),
        ])
    }

    /// Returns formatted resources rows
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
