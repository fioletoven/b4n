use b4n_config::keys::KeyCombination;
use b4n_kube::{Kind, Namespace};
use b4n_tui::{MouseEventKind, ResponseEvent, TuiEvent, utils::center};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use std::{rc::Rc, time::Instant};

use crate::core::{SharedAppData, SharedAppDataExt};
use crate::ui::presentation::content::edit::{ContentEditWidget, EditContext};
use crate::ui::presentation::content::header::ContentHeader;
use crate::ui::presentation::content::search::{ContentPosition, SearchData, SearchResultsWidget, get_search_wrapped_message};
use crate::ui::presentation::content::select::{ContentSelectWidget, SelectContext, Selection};
use crate::ui::presentation::{Content, StyledLineExt};

/// Content viewer with header.
pub struct ContentViewer<T: Content> {
    pub header: ContentHeader,
    app_data: SharedAppData,

    content: Option<T>,
    hash: Option<u64>,
    edit: EditContext,
    select: SelectContext,
    select_color: Color,
    search: SearchData,
    search_color: Color,

    page_start: ContentPosition,
    page_area: Rect,

    creation_time: Instant,
}

impl<T: Content> ContentViewer<T> {
    /// Creates a new content viewer.
    pub fn new(app_data: SharedAppData, select_color: Color, search_color: Color) -> Self {
        let header = ContentHeader::new(Rc::clone(&app_data), true);
        let cursor_color = app_data.borrow().theme.colors.cursor;

        Self {
            header,
            app_data,
            content: None,
            hash: None,
            edit: EditContext::new(cursor_color),
            select: SelectContext::default(),
            select_color,
            search: SearchData::default(),
            search_color,
            page_start: ContentPosition::default(),
            page_area: Rect::default(),
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
    pub fn set_content(&mut self, content: T) {
        self.content = Some(content);
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

    /// Clears the current selection.
    pub fn clear_selection(&mut self) {
        self.select.clear_selection();
    }

    /// Returns selection range if anything is selected.
    pub fn get_selection(&self) -> Option<Selection> {
        self.select.get_selection()
    }

    /// Returns `true` if there is selected text in the content.
    pub fn has_selection(&self) -> bool {
        self.content.is_some() && self.select.start.is_some() && self.select.end.is_some()
    }

    /// Returns `true` if viewer is in edit mode.
    pub fn is_in_edit_mode(&self) -> bool {
        self.edit.is_enabled
    }

    /// Returns `true` if content was altered.\
    /// **Note** that we must be still in the edit mode to actually run the check.
    pub fn is_modified(&self) -> bool {
        if !self.edit.is_enabled {
            return self.edit.is_modified;
        }

        match (self.hash, self.content.as_ref()) {
            (Some(hash), Some(content)) => content.hash() != hash,
            _ => false,
        }
    }

    /// Enables edit mode for the content viewer.
    pub fn enable_edit_mode(&mut self) -> bool {
        if self.edit.is_enabled {
            return false;
        }

        let search = self.current_match_position();
        if let Some(content) = &mut self.content
            && content.is_editable()
        {
            self.select.adjust_selection();
            self.edit.enable(
                self.page_start,
                self.select.get_selection(),
                search,
                self.page_area.height,
                content,
            );
            self.header.set_edit('', "[INS]  ");
            if self.hash.is_none()
                && let Some(content) = &self.content
            {
                self.hash = Some(content.hash());
            }

            self.scroll_to_cursor();
            self.disable_keys(true);
            true
        } else {
            false
        }
    }

    /// Disables edit mode for the content viewer.
    pub fn disable_edit_mode(&mut self) -> bool {
        if self.edit.is_enabled {
            self.edit.is_modified = self.is_modified(); // needs to be checked before setting is_enabled = false
            self.edit.is_enabled = false;
            if self.edit.is_modified {
                self.header.set_edit('!', "*  ");
            } else {
                self.header.set_edit(' ', "");
            }

            self.disable_keys(false);
            true
        } else {
            false
        }
    }

    /// Scrolls the view to the given `line` and `col` positions if they are outside the current viewport.
    pub fn scroll_to(&mut self, line: usize, col: usize, width: usize) {
        if line < self.page_start.y || line > self.page_start.y + usize::from(self.page_area.height.saturating_sub(1)) {
            let line = line.saturating_sub(self.page_area.height.saturating_div(2).into());
            self.page_start.y = line.min(self.max_vstart());
        }

        if col < self.page_start.x
            || col.saturating_add(width) > self.page_start.x + usize::from(self.page_area.width.saturating_sub(1))
        {
            let col = col.saturating_sub(self.page_area.width.saturating_div(2).into());
            self.page_start.x = col.min(self.max_hstart());
        }
    }

    /// Scrolls content to the current search match.
    pub fn scroll_to_current_match(&mut self, offset: Option<Position>) {
        if let Some(matches) = &self.search.matches {
            let offset = offset.unwrap_or_default();
            if let Some(current) = self.search.current {
                let current_match = &matches[current.saturating_sub(1)];
                self.scroll_to(
                    current_match.y.saturating_add(offset.y.into()),
                    current_match.x.saturating_add(offset.x.into()),
                    current_match.length,
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

    /// Scrolls to the current cursor position.
    pub fn scroll_to_cursor(&mut self) {
        self.scroll_to(self.edit.cursor.y, self.edit.cursor.x, 1);
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

    /// Returns currently highlighted match index.
    pub fn current_match_index(&self) -> Option<usize> {
        self.search.current
    }

    /// Returns currently highlighted match position.\
    /// **Note** that it returns first match if all are highlighted.
    pub fn current_match_position(&self) -> Option<ContentPosition> {
        let matches = self.search.matches.as_ref()?;
        let current = self.search.current.unwrap_or_default();
        let current = &matches[current.saturating_sub(1)];

        Some(ContentPosition {
            x: current.x,
            y: current.y,
        })
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

    /// Inserts specified text to the content at the current cursor position.\
    /// `on_line_start` - adds text at the start of the current line if text is a line from Ctrl+C/Ctrl+X.
    pub fn insert_text(&mut self, text: Vec<String>, on_line_start: bool) {
        if let Some(content) = &mut self.content {
            if let Some(range) = self.select.get_selection() {
                self.edit.cursor = range.start;
                content.remove_text(range);
            }

            self.select.clear_selection();
            let new_pos = if on_line_start && text.len() == 2 {
                content.insert_text(ContentPosition::new(0, self.edit.cursor.y), text);
                ContentPosition::new(self.edit.cursor.x, self.edit.cursor.y + 1)
            } else {
                content.insert_text(self.edit.cursor, text)
            };
            self.edit.cursor = new_pos;
        }
    }

    /// Returns text of the line under the cursor.\
    /// **Note** that it works only in edit mode (when the cursor is visible).
    pub fn get_current_line(&self) -> Option<&str> {
        if self.edit.is_enabled {
            self.content()?.line(self.edit.cursor.y)
        } else {
            None
        }
    }

    /// Returns currently visible lines.
    pub fn get_page_lines(&mut self) -> Vec<Line<'_>> {
        let start = self.page_start.y.clamp(0, self.max_vstart());
        self.content
            .as_mut()
            .unwrap()
            .page(start, self.page_area.height.into())
            .iter()
            .map(|line| line.as_line(self.page_start.x))
            .collect::<Vec<_>>()
    }

    /// Gets footer icon text for the current search state.
    pub fn get_footer_text(&self) -> Option<String> {
        if let Some(count) = self.matches_count() {
            if let Some(current) = self.current_match_index() {
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
        if self.matches_count().is_some() && self.current_match_index().is_some_and(|c| c == 0) {
            Some(get_search_wrapped_message(forward))
        } else {
            None
        }
    }

    /// Allows content to process some computation on app tick.
    pub fn process_tick(&mut self) -> ResponseEvent {
        if let Some(content) = &mut self.content {
            content.process_tick()
        } else {
            ResponseEvent::Handled
        }
    }

    /// Process UI key/mouse event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let Some(content) = &mut self.content {
            let cursor = if self.edit.is_enabled { Some(self.edit.cursor) } else { None };
            self.select
                .process_event(event, content, &mut self.page_start, cursor, self.page_area);

            if self.edit.is_enabled {
                let response =
                    self.edit
                        .process_event(event, content, self.page_start, self.select.get_selection(), self.page_area);
                if response != ResponseEvent::NotHandled {
                    self.select.process_event_final(event, content, self.edit.cursor);
                    let (y, x) = (self.edit.cursor.y, self.edit.cursor.x);
                    self.scroll_to(y, x, 1);
                    return response;
                }
            }
        }

        match event {
            TuiEvent::Key(key) => {
                match key {
                    // horizontal scroll
                    x if x.code == KeyCode::Home && x.modifiers == KeyModifiers::CONTROL => self.page_start.x = 0,
                    x if x.code == KeyCode::PageUp && x.modifiers == KeyModifiers::CONTROL => {
                        self.page_start.x = self.page_start.x.saturating_sub(self.page_area.width.into());
                    },
                    x if x.code == KeyCode::Left => self.page_start.x = self.page_start.x.saturating_sub(1),
                    x if x.code == KeyCode::Right => self.page_start.x += 1,
                    x if x.code == KeyCode::PageDown && x.modifiers == KeyModifiers::CONTROL => {
                        self.page_start.x += usize::from(self.page_area.width);
                    },
                    x if x.code == KeyCode::End && x.modifiers == KeyModifiers::CONTROL => self.page_start.x = self.max_hstart(),

                    // vertical scroll
                    x if x.code == KeyCode::Home => self.page_start.y = 0,
                    x if x.code == KeyCode::PageUp => {
                        self.page_start.y = self.page_start.y.saturating_sub(self.page_area.height.into());
                    },
                    x if x.code == KeyCode::Up => self.page_start.y = self.page_start.y.saturating_sub(1),
                    x if x.code == KeyCode::Down => self.page_start.y += 1,
                    x if x.code == KeyCode::PageDown => self.page_start.y += usize::from(self.page_area.height),
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

    /// Draws the [`ContentViewer`] onto the given frame within the specified area.\
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
        self.page_area = area;
        self.update_page_start();

        frame.render_widget(Paragraph::new(self.get_page_lines()), area);
        frame.render_widget(
            ContentSelectWidget::new(
                &self.select,
                self.content.as_ref().unwrap(),
                &self.page_start,
                self.select_color,
            ),
            area,
        );

        if self.search.matches.is_some() {
            let widget = SearchResultsWidget::new(self.page_start, &self.search, self.search_color).with_offset(highlight_offset);
            frame.render_widget(widget, area);
        }

        if self.edit.is_enabled {
            frame.render_widget(ContentEditWidget::new(&self.edit, &self.page_start), area);
        }
    }

    fn draw_empty(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.page_area = area;
        let colors = &self.app_data.borrow().theme.colors;
        let line = Line::styled(" waiting for data…", &colors.text);
        let area = center(area, Constraint::Length(line.width() as u16), Constraint::Length(4));
        frame.render_widget(line, area);
    }

    /// Returns max vertical start of the page.
    fn max_vstart(&self) -> usize {
        self.content.as_ref().map_or(0, |c| c.max_vstart(self.page_area.height))
    }

    /// Returns max horizontal start of the page.
    fn max_hstart(&self) -> usize {
        self.content.as_ref().map_or(0, |c| c.max_hstart(self.page_area.width))
    }

    fn update_page_start(&mut self) {
        if self.page_start.y > self.max_vstart() {
            self.page_start.y = self.max_vstart();
        }

        if self.page_start.x > self.max_hstart() {
            self.page_start.x = self.max_hstart();
        }

        if self.edit.is_enabled {
            self.header.set_coordinates(self.edit.cursor.x, self.edit.cursor.y);
        } else {
            self.header.set_coordinates(self.page_start.x, self.page_start.y);
        }
    }

    fn disable_keys(&self, is_disabled: bool) {
        for ch in ['x', 'c', 'v', 'd', 'y', 'z'] {
            self.app_data
                .disable_key(KeyCombination::new(KeyCode::Char(ch), KeyModifiers::CONTROL), is_disabled);
        }
    }
}

impl<T: Content> Drop for ContentViewer<T> {
    fn drop(&mut self) {
        self.disable_keys(false);
    }
}
