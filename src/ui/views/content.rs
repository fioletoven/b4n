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

pub type StyledLine = Vec<(Style, String)>;

/// Content for the [`ContentViewer`].
pub trait Content {
    /// Returns page with [`StyledLine`]s.
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine];

    /// Returns the current length of a [`Column`].
    fn len(&self) -> usize;
}

/// Content viewer with header.
pub struct ContentViewer<T: Content> {
    pub header: HeaderPane,
    app_data: SharedAppData,

    content: Option<T>,
    max_width: usize,

    page_height: usize,
    page_width: usize,
    page_vstart: usize,
    page_hstart: usize,

    creation_time: Instant,
}

impl<T: Content> ContentViewer<T> {
    /// Creates a new content viewer.
    pub fn new(app_data: SharedAppData) -> Self {
        let header = HeaderPane::new(Rc::clone(&app_data));

        Self {
            header,
            app_data,
            content: None,
            max_width: 0,
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

    /// Returns `true` if viewer has content.
    pub fn has_content(&self) -> bool {
        self.content.is_some()
    }

    /// Sets content for the viewer.
    pub fn set_content(&mut self, content: T, max_width: usize) {
        self.content = Some(content);
        self.max_width = max_width;
    }

    /// Returns content as mutable reference.
    pub fn content_mut(&mut self) -> Option<&mut T> {
        self.content.as_mut()
    }

    /// Updates max width for content lines.
    pub fn update_max_width(&mut self, max_width: usize) {
        if self.max_width < max_width {
            self.max_width = max_width;
        }
    }

    /// Updates page height.
    pub fn update_page(&mut self, new_height: u16, hew_width: u16) {
        self.page_height = usize::from(new_height);
        self.page_width = usize::from(hew_width);
        self.update_page_starts();
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

        if self.content.is_some() {
            self.update_page(layout[1].height, layout[1].width);

            let start = self.page_vstart.clamp(0, self.max_vstart());
            let lines = self
                .content
                .as_mut()
                .unwrap()
                .page(start, usize::from(layout[1].height))
                .iter()
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
        self.content
            .as_ref()
            .map(|l| l.len().saturating_sub(self.page_height))
            .unwrap_or(0)
    }

    /// Returns max horizontal start of the page.
    fn max_hstart(&self) -> usize {
        self.max_width.saturating_sub(self.page_width)
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
