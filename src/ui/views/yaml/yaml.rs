use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::{
    app::SharedAppData,
    kubernetes::Namespace,
    ui::{widgets::Footer, ResponseEvent},
};

use super::HeaderPane;

/// YAML viewer with header and footer.
pub struct YamlViewer {
    pub header: HeaderPane,
    pub footer: Footer,
    lines: Vec<Vec<(Style, String)>>,
    page_start: usize,
    page_height: usize,
}

impl YamlViewer {
    /// Creates a new YAML viewer page.
    pub fn new(app_data: SharedAppData, name: String, namespace: Namespace, kind_plural: String) -> Self {
        let header = HeaderPane::new(Rc::clone(&app_data), name, namespace, kind_plural);
        let footer = Footer::new(Rc::clone(&app_data));

        Self {
            header,
            footer,
            lines: Vec::new(),
            page_start: 0,
            page_height: 0,
        }
    }

    /// Sets header data.
    pub fn set_header(&mut self, name: String, namespace: Namespace, kind_plural: String) {
        self.header.set_data(name, namespace, kind_plural);
    }

    /// Sets styled YAML content to view.
    pub fn set_content(&mut self, styled_lines: Vec<Vec<(Style, String)>>) {
        self.lines = styled_lines;
    }

    /// Updates page height.
    pub fn update_page(&mut self, new_height: u16) {
        self.page_height = usize::from(new_height);
    }

    /// Returns max start value.
    fn max_start(&self) -> usize {
        self.lines.len().saturating_sub(self.page_height)
    }

    /// Process UI key event.
    pub fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if key.code == KeyCode::Home {
            self.page_start = 0;
        }

        if key.code == KeyCode::PageUp {
            self.page_start = self.page_start.saturating_sub(self.page_height);
        }

        if key.code == KeyCode::Up {
            if self.page_start > 0 {
                self.page_start -= 1;
            }
        }

        if key.code == KeyCode::Down {
            if self.page_start < self.max_start() {
                self.page_start += 1;
            }
        }

        if key.code == KeyCode::PageDown {
            self.page_start += self.page_height;
            if self.page_start > self.max_start() {
                self.page_start = self.max_start();
            }
        }

        if key.code == KeyCode::End {
            self.page_start = self.max_start();
        }

        ResponseEvent::Handled
    }

    /// Draws [`YamlViewer`] on the provided frame and area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)])
            .split(area);

        self.update_page(layout[1].height);

        let start = self.page_start.clamp(0, self.max_start());
        let lines = self
            .lines
            .iter()
            .skip(start)
            .take(usize::from(layout[1].height))
            .map(|items| Line::from(items.iter().map(|item| Span::styled(&item.1, item.0)).collect::<Vec<_>>()))
            .collect::<Vec<_>>();

        self.header.draw(frame, layout[0]);
        frame.render_widget(Paragraph::new(lines), layout[1]);
        self.footer.draw(frame, layout[2]);
    }
}
