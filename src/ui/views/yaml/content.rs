use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use std::{rc::Rc, time::Instant};

use crate::{
    app::SharedAppData,
    kubernetes::Namespace,
    ui::{ResponseEvent, utils::center},
};

use super::HeaderPane;

/// YAML content with header.
pub struct YamlContent {
    pub header: HeaderPane,
    app_data: SharedAppData,

    lines: Vec<Vec<(Style, String)>>,
    lines_width: usize,

    page_height: usize,
    page_width: usize,
    page_vstart: usize,
    page_hstart: usize,

    has_content: bool,
    creation_time: Instant,
}

impl YamlContent {
    /// Creates a new YAML viewer page.
    pub fn new(app_data: SharedAppData, name: String, namespace: Namespace, kind_plural: String, is_decoded: bool) -> Self {
        let header = HeaderPane::new(Rc::clone(&app_data), name, namespace, kind_plural, is_decoded);

        Self {
            header,
            app_data,
            lines: Vec::new(),
            lines_width: 0,
            page_height: 0,
            page_width: 0,
            page_vstart: 0,
            page_hstart: 0,
            has_content: false,
            creation_time: Instant::now(),
        }
    }

    /// Sets header data.
    pub fn set_header(&mut self, name: String, namespace: Namespace, kind_plural: String, is_decoded: bool) {
        self.header.set_data(name, namespace, kind_plural, is_decoded);
    }

    /// Sets styled YAML content.
    pub fn set_content(&mut self, styled_lines: Vec<Vec<(Style, String)>>, max_width: usize) {
        self.lines = styled_lines;
        self.lines_width = max_width;
        self.has_content = true;
    }

    /// Updates page height.
    pub fn update_page(&mut self, new_height: u16, hew_width: u16) {
        self.page_height = usize::from(new_height);
        self.page_width = usize::from(hew_width);
        self.update_page_starts();
    }

    /// Returns max vertical start of the page.
    fn max_vstart(&self) -> usize {
        self.lines.len().saturating_sub(self.page_height)
    }

    /// Returns max horizontal start of the page.
    fn max_hstart(&self) -> usize {
        self.lines_width.saturating_sub(self.page_width)
    }

    /// Process UI key event.
    pub fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        match key {
            // horizontal scroll
            x if x.code == KeyCode::Home && x.modifiers == KeyModifiers::SHIFT => self.page_hstart = 0,
            x if x.code == KeyCode::PageUp && x.modifiers == KeyModifiers::SHIFT => {
                self.page_hstart = self.page_hstart.saturating_sub(self.page_width)
            }
            x if x.code == KeyCode::Left => self.page_hstart = self.page_hstart.saturating_sub(1),
            x if x.code == KeyCode::Right => self.page_hstart += 1,
            x if x.code == KeyCode::PageDown && x.modifiers == KeyModifiers::SHIFT => self.page_hstart += self.page_width,
            x if x.code == KeyCode::End && x.modifiers == KeyModifiers::SHIFT => self.page_hstart = self.max_hstart(),

            // vertical scroll
            x if x.code == KeyCode::Home => self.page_vstart = 0,
            x if x.code == KeyCode::PageUp => self.page_vstart = self.page_vstart.saturating_sub(self.page_height),
            x if x.code == KeyCode::Up => self.page_vstart = self.page_vstart.saturating_sub(1),
            x if x.code == KeyCode::Down => self.page_vstart += 1,
            x if x.code == KeyCode::PageDown => self.page_vstart += self.page_height,
            x if x.code == KeyCode::End => self.page_vstart = self.max_vstart(),

            _ => (),
        }

        self.update_page_starts();
        ResponseEvent::Handled
    }

    /// Draws [`YamlContent`] on the provided frame and area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        self.header.draw(frame, layout[0]);

        if self.has_content {
            self.update_page(layout[1].height, layout[1].width);

            let start = self.page_vstart.clamp(0, self.max_vstart());
            let lines = self
                .lines
                .iter()
                .skip(start)
                .take(usize::from(layout[1].height))
                .map(|items| Line::from(items.iter().map(|item| Span::styled(&item.1, item.0)).collect::<Vec<_>>()))
                .collect::<Vec<_>>();

            frame.render_widget(Paragraph::new(lines).scroll((0, self.page_hstart as u16)), layout[1]);
        } else if self.creation_time.elapsed().as_millis() > 80 {
            let colors = &self.app_data.borrow().theme.colors;
            let line = Line::styled(" waiting for data…", &colors.text);
            let area = center(area, Constraint::Length(line.width() as u16), Constraint::Length(4));
            frame.render_widget(line, area);
        }
    }

    fn update_page_starts(&mut self) {
        if self.page_vstart > self.max_vstart() {
            self.page_vstart = self.max_vstart();
        }

        if self.page_hstart > self.max_hstart() {
            self.page_hstart = self.max_hstart();
        }

        self.header.set_coordinates(self.page_hstart, self.page_vstart);
    }
}
