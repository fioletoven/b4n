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

use super::header::HeaderPane;

pub type StyledLines = Vec<Vec<(Style, String)>>;

/// Content viewer with header.
pub struct ContentViewer {
    pub header: HeaderPane,
    app_data: SharedAppData,

    lines: Option<StyledLines>,
    lines_width: usize,

    page_height: usize,
    page_width: usize,
    page_vstart: usize,
    page_hstart: usize,

    creation_time: Instant,
}

impl ContentViewer {
    /// Creates a new content viewer.
    pub fn new(app_data: SharedAppData) -> Self {
        let header = HeaderPane::new(Rc::clone(&app_data));

        Self {
            header,
            app_data,
            lines: None,
            lines_width: 0,
            page_height: 0,
            page_width: 0,
            page_vstart: 0,
            page_hstart: 0,
            creation_time: Instant::now(),
        }
    }

    /// Sets header data.
    pub fn with_header(
        mut self,
        title: &'static str,
        icon: char,
        namespace: Namespace,
        kind: String,
        name: String,
        descr: Option<String>,
    ) -> Self {
        self.header.set_title(title);
        self.header.set_icon(icon);
        self.header.set_data(namespace, kind, name, descr);
        self
    }

    /// Sets header data.
    pub fn set_header_data(&mut self, namespace: Namespace, kind: String, name: String, descr: Option<String>) {
        self.header.set_data(namespace, kind, name, descr);
    }

    /// Sets header title.
    pub fn set_header_title(&mut self, title: &'static str) {
        self.header.set_title(title);
    }

    /// Sets header icon.
    pub fn set_header_icon(&mut self, icon: char) {
        self.header.set_icon(icon);
    }

    /// Returns `true` if viewer has content.
    pub fn has_content(&self) -> bool {
        self.lines.is_some()
    }

    /// Sets styled content.
    pub fn set_content(&mut self, styled_lines: StyledLines, max_width: usize) {
        self.lines = Some(styled_lines);
        self.lines_width = max_width;
    }

    /// Returns styled content as mutable reference.
    pub fn content_mut(&mut self) -> Option<&mut StyledLines> {
        self.lines.as_mut()
    }

    /// Updates max width for content lines.
    pub fn update_max_width(&mut self, max_width: usize) {
        if self.lines_width < max_width {
            self.lines_width = max_width;
        }
    }

    /// Updates page height.
    pub fn update_page(&mut self, new_height: u16, hew_width: u16) {
        self.page_height = usize::from(new_height);
        self.page_width = usize::from(hew_width);
        self.update_page_starts();
    }

    /// Scrolls content to the end.
    pub fn scroll_to_start(&mut self) {
        self.page_vstart = 0;
    }

    /// Scrolls content to the end.
    pub fn scroll_to_end(&mut self) {
        self.page_vstart = self.max_vstart();
    }

    /// Returns `true` if view is showing the last part of the content.
    pub fn is_at_end(&self) -> bool {
        self.page_vstart == self.max_vstart()
    }

    /// Process UI key event.
    pub fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        match key {
            // horizontal scroll
            x if x.code == KeyCode::Home && x.modifiers == KeyModifiers::SHIFT => self.page_hstart = 0,
            x if x.code == KeyCode::PageUp && x.modifiers == KeyModifiers::SHIFT => {
                self.page_hstart = self.page_hstart.saturating_sub(self.page_width)
            },
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

            _ => return ResponseEvent::NotHandled,
        }

        self.update_page_starts();
        ResponseEvent::Handled
    }

    /// Draws [`ContentViewer`] on the provided frame and area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        self.header.draw(frame, layout[0]);

        if self.lines.is_some() {
            self.update_page(layout[1].height, layout[1].width);

            let start = self.page_vstart.clamp(0, self.max_vstart());
            let lines = self
                .lines
                .as_ref()
                .unwrap()
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

    /// Returns max vertical start of the page.
    fn max_vstart(&self) -> usize {
        self.lines
            .as_ref()
            .map(|l| l.len().saturating_sub(self.page_height))
            .unwrap_or(0)
    }

    /// Returns max horizontal start of the page.
    fn max_hstart(&self) -> usize {
        self.lines_width.saturating_sub(self.page_width)
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
