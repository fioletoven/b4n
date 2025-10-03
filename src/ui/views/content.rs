use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::{rc::Rc, time::Instant};

use crate::{
    core::SharedAppData,
    kubernetes::{Kind, Namespace},
    ui::{
        MouseEventKind, ResponseEvent, TuiEvent,
        utils::center,
        views::{
            content_edit::{ContentEditWidget, EditContext},
            content_search::{MatchPosition, PagePosition, SearchData, SearchResultsWidget, get_search_wrapped_message},
        },
    },
};

use super::content_header::ContentHeader;

pub type StyledLine = Vec<(Style, String)>;

/// Content for the [`ContentViewer`].
pub trait Content {
    /// Returns page with [`StyledLine`]s.
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine];

    /// Returns the length of a [`Content`].
    fn len(&self) -> usize;

    /// Searches content for the specified pattern.
    fn search(&self, pattern: &str) -> Vec<MatchPosition>;

    /// Returns characters count of the line under `line_no` index.
    fn line_size(&self, line_no: usize) -> usize;
}

/// Content viewer with header.
pub struct ContentViewer<T: Content> {
    pub header: ContentHeader,
    app_data: SharedAppData,

    content: Option<T>,
    edit: Option<EditContext>,
    search: SearchData,
    search_color: Color,
    max_width: usize,

    page_height: usize,
    page_width: usize,
    page_start: PagePosition,

    creation_time: Instant,
}

impl<T: Content> ContentViewer<T> {
    /// Creates a new content viewer.
    pub fn new(app_data: SharedAppData, search_color: Color) -> Self {
        let header = ContentHeader::new(Rc::clone(&app_data), true);

        Self {
            header,
            app_data,
            content: None,
            edit: None,
            search: SearchData::default(),
            search_color,
            max_width: 0,
            page_height: 0,
            page_width: 0,
            page_start: PagePosition::default(),
            creation_time: Instant::now(),
        }
    }

    /// Sets header data.
    pub fn with_header(
        mut self,
        title: &'static str,
        icon: char,
        namespace: Namespace,
        kind: Kind,
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
        self.search = SearchData::default();
    }

    /// Returns content as reference.
    pub fn content(&self) -> Option<&T> {
        self.content.as_ref()
    }

    /// Returns content as mutable reference.
    pub fn content_mut(&mut self) -> Option<&mut T> {
        self.content.as_mut()
    }

    /// Adds value to the maximum width of content lines.
    pub fn max_width_add(&mut self, rhs: usize) {
        self.max_width = self.max_width.saturating_add(rhs);
    }

    /// Subtracts value from the maximum width of content lines.
    pub fn max_width_sub(&mut self, rhs: usize) {
        self.max_width = self.max_width.saturating_sub(rhs);
    }

    /// Updates max width for content lines.\
    /// **Note** that it will not update the width if new one is smaller.
    pub fn update_max_width(&mut self, new_max_width: usize) {
        if self.max_width < new_max_width {
            self.max_width = new_max_width;
        }
    }

    /// Updates page `height` and `width`.
    pub fn update_page(&mut self, new_height: u16, hew_width: u16) {
        self.page_height = usize::from(new_height);
        self.page_width = usize::from(hew_width);
        self.update_page_start();
    }

    /// Scrolls the view to the given `line` and `col` positions if they are outside the current viewport.
    pub fn scroll_to(&mut self, line: usize, col: usize, width: usize) {
        if line < self.page_start.y || line > self.page_start.y + self.page_height.saturating_sub(1) {
            let line = line.saturating_sub(self.page_height.saturating_div(2));
            self.page_start.y = line.min(self.max_vstart());
        }

        if col < self.page_start.x || col.saturating_add(width) > self.page_start.x + self.page_width.saturating_sub(1) {
            let col = col.saturating_sub(self.page_width.saturating_div(2));
            self.page_start.x = col.min(self.max_hstart());
        }
    }

    /// Scrolls content to the current search match.
    pub fn scroll_to_current_match(&mut self, offset: Option<Position>) {
        if let Some(matches) = &self.search.matches {
            let offset = offset.unwrap_or_default();
            if let Some(current) = self.search.current {
                let r#match = &matches[current.saturating_sub(1)];
                self.scroll_to(
                    r#match.y.saturating_add(offset.y.into()),
                    r#match.x.saturating_add(offset.x.into()),
                    r#match.length,
                );
            } else if !matches.is_empty() {
                self.scroll_to(
                    matches[0].y.saturating_add(offset.y.into()),
                    matches[0].x.saturating_add(offset.x.into()),
                    matches[0].length,
                );
            }
        }
    }

    /// Scrolls content to the end.
    pub fn scroll_to_end(&mut self) {
        self.page_start.y = self.max_vstart();
    }

    /// Returns `true` if view is showing the last part of the content.
    pub fn is_at_end(&self) -> bool {
        self.page_start.y == self.max_vstart()
    }

    /// Resets horizontal scroll to start position.
    pub fn reset_horizontal_scroll(&mut self) {
        self.page_start.x = 0;
    }

    /// Searches content for the specified pattern.\
    /// Returns `true` if the search was updated.
    pub fn search(&mut self, pattern: &str, force: bool) -> bool {
        let is_pattern_changed = self.search.pattern.as_ref().is_none_or(|p| p != pattern);
        if let Some(content) = &self.content
            && (force || is_pattern_changed)
        {
            if pattern.is_empty() {
                self.search = SearchData::default();
            } else {
                self.search.pattern = Some(pattern.to_owned());
                let matches = content.search(pattern);
                if is_pattern_changed || self.search.current.unwrap_or_default() > matches.len() {
                    self.search.current = None;
                }
                if matches.is_empty() {
                    self.search.matches = Some(Vec::default());
                } else {
                    self.search.matches = Some(matches);
                }
            }

            true
        } else {
            false
        }
    }

    /// Returns the number of search matches.
    pub fn matches_count(&self) -> Option<usize> {
        self.search.matches.as_ref().map(Vec::len)
    }

    /// Returns currently highlighted match.
    pub fn current_match(&self) -> Option<usize> {
        self.search.current
    }

    /// Updates the current match index in the search results based on navigation direction.\
    /// **Note** that updated index will start from 1.
    pub fn navigate_match(&mut self, forward: bool, offset: Option<Position>) {
        let total = self.search.matches.as_ref().map_or(0, Vec::len);
        if total == 0 {
            return;
        }

        if total > 1 {
            self.search.current = match self.search.current {
                Some(current) => {
                    if forward {
                        let current = current.saturating_add(1);
                        if current > total { None } else { Some(current) }
                    } else {
                        let current = current.saturating_sub(1);
                        if current == 0 { None } else { Some(current) }
                    }
                },
                None => Some(if forward { 1 } else { total }),
            };
        }

        if self.search.current.is_some() {
            self.scroll_to_current_match(offset);
        } else if total == 1
            && let Some(matches) = self.search.matches.as_ref()
        {
            let offset = offset.unwrap_or_default();
            self.scroll_to(
                matches[0].y.saturating_add(offset.y.into()),
                matches[0].x.saturating_add(offset.x.into()),
                matches[0].length,
            );
        }
    }

    pub fn get_page_lines(&mut self) -> Vec<Line<'_>> {
        let start = self.page_start.y.clamp(0, self.max_vstart());
        self.content
            .as_mut()
            .unwrap()
            .page(start, self.page_height)
            .iter()
            .map(|items| Line::from(items.iter().map(|item| Span::styled(&item.1, item.0)).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    }

    /// Gets footer icon text for the current search state.
    pub fn get_footer_text(&self) -> Option<String> {
        if let Some(count) = self.matches_count() {
            if let Some(current) = self.current_match() {
                Some(format!(" {current}:{count}"))
            } else if count == 0 {
                Some(format!(" {count}"))
            } else {
                Some(format!(" :{count}"))
            }
        } else {
            None
        }
    }

    /// Gets footer message for the current search state.
    pub fn get_footer_message(&self, forward: bool) -> Option<&'static str> {
        if self.matches_count().is_some() && self.current_match().is_some_and(|c| c == 0) {
            Some(get_search_wrapped_message(forward))
        } else {
            None
        }
    }

    /// Process UI key/mouse event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let Some(edit) = &mut self.edit
            && let Some(content) = &self.content
        {
            let response = edit.process_event(event, content);
            if response != ResponseEvent::NotHandled {
                return response;
            }
        }

        match event {
            TuiEvent::Key(key) => {
                match key {
                    x if x.code == KeyCode::Char('i') => self.enable_edit_mode(),

                    // horizontal scroll
                    x if x.code == KeyCode::Home && x.modifiers == KeyModifiers::CONTROL => self.page_start.x = 0,
                    x if x.code == KeyCode::PageUp && x.modifiers == KeyModifiers::CONTROL => {
                        self.page_start.x = self.page_start.x.saturating_sub(self.page_width);
                    },
                    x if x.code == KeyCode::Left => self.page_start.x = self.page_start.x.saturating_sub(1),
                    x if x.code == KeyCode::Right => self.page_start.x += 1,
                    x if x.code == KeyCode::PageDown && x.modifiers == KeyModifiers::CONTROL => {
                        self.page_start.x += self.page_width;
                    },
                    x if x.code == KeyCode::End && x.modifiers == KeyModifiers::CONTROL => self.page_start.x = self.max_hstart(),

                    // vertical scroll
                    x if x.code == KeyCode::Home => self.page_start.y = 0,
                    x if x.code == KeyCode::PageUp => self.page_start.y = self.page_start.y.saturating_sub(self.page_height),
                    x if x.code == KeyCode::Up => self.page_start.y = self.page_start.y.saturating_sub(1),
                    x if x.code == KeyCode::Down => self.page_start.y += 1,
                    x if x.code == KeyCode::PageDown => self.page_start.y += self.page_height,
                    x if x.code == KeyCode::End => self.page_start.y = self.max_vstart(),

                    _ => return ResponseEvent::NotHandled,
                }
            },
            TuiEvent::Mouse(mouse) => match mouse {
                // horizontal scroll
                x if x.kind == MouseEventKind::ScrollUp && x.modifiers == KeyModifiers::CONTROL => {
                    self.page_start.x = self.page_start.x.saturating_sub(1);
                },
                x if x.kind == MouseEventKind::ScrollDown && x.modifiers == KeyModifiers::CONTROL => self.page_start.x += 1,
                x if x.kind == MouseEventKind::ScrollLeft => self.page_start.x = self.page_start.x.saturating_sub(1),
                x if x.kind == MouseEventKind::ScrollRight => self.page_start.x += 1,

                // vertical scroll
                x if x.kind == MouseEventKind::ScrollUp => self.page_start.y = self.page_start.y.saturating_sub(1),
                x if x.kind == MouseEventKind::ScrollDown => self.page_start.y += 1,

                _ => return ResponseEvent::NotHandled,
            },
        }

        self.update_page_start();
        ResponseEvent::Handled
    }

    /// Draws the [`ContentViewer`] onto the given frame within the specified area.
    ///
    /// `highlight_offset` - used to adjust the position of search highlights.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, highlight_offset: Option<Position>) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        self.header.draw(frame, layout[0]);
        frame.render_widget(Block::new().style(&self.app_data.borrow().theme.colors.text), layout[1]);

        if self.content.is_some() {
            self.draw_content(frame, layout[1], highlight_offset);
        } else if self.creation_time.elapsed().as_millis() > 80 {
            self.draw_empty(frame, area);
        }
    }

    fn draw_content(&mut self, frame: &mut Frame<'_>, area: Rect, highlight_offset: Option<Position>) {
        let area = area.inner(Margin::new(1, 0));
        self.update_page(area.height, area.width);

        let hscroll = u16::try_from(self.page_start.x).unwrap_or_default();
        let lines = self.get_page_lines();
        frame.render_widget(Paragraph::new(lines).scroll((0, hscroll)), area);

        if self.search.matches.is_some() {
            frame.render_widget(
                SearchResultsWidget::new(self.page_start, &self.search, self.search_color).with_offset(highlight_offset),
                area,
            );
        }

        if let Some(edit) = &self.edit
            && let Some(content) = &self.content
        {
            frame.render_widget(ContentEditWidget::new(content, edit, &self.page_start), area);
        }
    }

    fn draw_empty(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let colors = &self.app_data.borrow().theme.colors;
        let line = Line::styled(" waiting for data…", &colors.text);
        let area = center(area, Constraint::Length(line.width() as u16), Constraint::Length(4));
        frame.render_widget(line, area);
    }

    /// Returns max vertical start of the page.
    fn max_vstart(&self) -> usize {
        self.content.as_ref().map_or(0, |l| l.len().saturating_sub(self.page_height))
    }

    /// Returns max horizontal start of the page.
    fn max_hstart(&self) -> usize {
        self.max_width.saturating_sub(self.page_width)
    }

    fn enable_edit_mode(&mut self) {
        if self.edit.is_some() || self.content.is_none() {
            return;
        }

        self.edit = Some(EditContext::new());
    }

    fn update_page_start(&mut self) {
        if self.page_start.y > self.max_vstart() {
            self.page_start.y = self.max_vstart();
        }

        if self.page_start.x > self.max_hstart() {
            self.page_start.x = self.max_hstart();
        }

        self.header.set_coordinates(self.page_start.x, self.page_start.y);
    }
}
