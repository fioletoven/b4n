use b4n_common::{IconKind, NotificationSink, slice_from, slice_to, substring};
use b4n_config::keys::KeyCommand;
use b4n_config::themes::LogsSyntaxColors;
use b4n_kube::client::KubernetesClient;
use b4n_kube::{PODS, PodRef, ResourceRef};
use b4n_tui::widgets::{ActionItem, ActionsListBuilder};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent};
use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::ui::presentation::{Content, ContentPosition, ContentViewer, MatchPosition, Selection, StyledLine};
use crate::ui::views::View;
use crate::ui::widgets::{CommandPalette, Search};

use super::{LogLine, LogsObserver, LogsObserverError};

const INITIAL_LOGS_VEC_SIZE: usize = 5_000;
const TIMESTAMP_TEXT_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f ";
const TIMESTAMP_TEXT_LENGTH: usize = 24;

/// Logs view.
pub struct LogsView {
    logs: ContentViewer<LogsContent>,
    app_data: SharedAppData,
    observer: LogsObserver,
    last_mouse_click: Option<Position>,
    command_palette: CommandPalette,
    search: Search,
    footer: NotificationSink,
    bound_to_bottom: bool,
    area: Rect,
}

impl LogsView {
    /// Creates new [`LogsView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        client: &KubernetesClient,
        resource: ResourceRef,
        previous: bool,
        footer: NotificationSink,
        workspace: Rect,
    ) -> Result<Self, LogsObserverError> {
        let pod = PodRef {
            name: resource.name.clone().unwrap_or_default(),
            namespace: resource.namespace.clone(),
            container: resource.container.clone(),
        };
        let select = app_data.borrow().theme.colors.syntax.logs.select;
        let search = app_data.borrow().theme.colors.syntax.logs.search;
        let area = ContentViewer::<LogsContent>::get_content_area(workspace);
        let logs = ContentViewer::new(Rc::clone(&app_data), select, search, area).with_header(
            if previous { "previous logs" } else { "logs" },
            '',
            resource.namespace,
            PODS.into(),
            resource.name,
            resource.container,
        );

        let mut observer = LogsObserver::new(worker.borrow().runtime_handle().clone());
        observer.start(client, pod, app_data.borrow().config.logs.lines, previous);
        let search = Search::new(Rc::clone(&app_data), Some(worker), 60);

        Ok(Self {
            logs,
            app_data,
            observer,
            last_mouse_click: None,
            command_palette: CommandPalette::default(),
            search,
            footer,
            bound_to_bottom: true,
            area: workspace,
        })
    }

    fn show_command_palette(&mut self) {
        let builder = ActionsListBuilder::default()
            .with_back()
            .with_quit()
            .with_action(ActionItem::action("timestamps", "timestamps").with_description("toggles the display of timestamps"))
            .with_action(ActionItem::action("copy", "copy").with_description("copies logs to the clipboard"))
            .with_action(ActionItem::action("search", "search").with_description("searches logs using the provided query"));
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60)
            .with_highlighted_position(self.last_mouse_click.take());
        self.command_palette.show();
    }

    fn show_mouse_menu(&mut self, x: u16, y: u16) {
        let copy = if self.logs.has_selection() { "selection" } else { "all" };
        let builder = ActionsListBuilder::default()
            .with_action(ActionItem::back())
            .with_action(ActionItem::command_palette())
            .with_action(ActionItem::menu(1, &format!("󰆏 copy [{copy}]"), "copy"))
            .with_action(ActionItem::menu(2, " search", "search"));
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 22).as_mouse_menu();
        self.command_palette.show_at(x.saturating_sub(1), y);
    }

    fn toggle_timestamps(&mut self) {
        self.logs.clear_selection();
        if let Some(content) = self.logs.content_mut() {
            content.toggle_timestamps();
            if content.show_timestamps {
                content.max_size = content.max_size.saturating_add(TIMESTAMP_TEXT_LENGTH);
            } else {
                content.max_size = content.max_size.saturating_sub(TIMESTAMP_TEXT_LENGTH);
            }

            self.logs.reset_horizontal_scroll();
        }
    }

    fn copy_logs_to_clipboard(&self) {
        if self.logs.content().is_some() {
            let range = self.logs.get_selection();
            if let Some(clipboard) = &mut self.app_data.borrow_mut().clipboard
                && clipboard
                    .set_text(self.logs.content().map(|c| c.to_plain_text(range)).unwrap_or_default())
                    .is_ok()
            {
                if self.logs.has_selection() {
                    self.footer.show_info(" Selection copied to clipboard…", 1_500);
                } else {
                    self.footer.show_info(" Container logs copied to clipboard…", 1_500);
                }
            } else {
                self.footer.show_error(" Unable to access clipboard functionality…", 2_000);
            }
        }
    }

    fn update_bound_to_bottom(&mut self) {
        self.bound_to_bottom = self.search.value().is_empty() && self.logs.is_at_end();
        self.logs.header.set_icon(if self.bound_to_bottom { '' } else { '' });
    }

    fn clear_search(&mut self) {
        self.logs.search("", false);
        self.search.reset();
        self.update_search_count();
        self.update_bound_to_bottom();
    }

    fn update_search_count(&mut self) {
        self.footer
            .set_text("900_logs_search", self.logs.get_footer_text(), IconKind::Default);
        self.search.set_matches(self.logs.matches_count());
    }

    fn navigate_match(&mut self, forward: bool) {
        self.logs.navigate_match(forward, self.get_offset());
        self.footer
            .set_text("900_logs_search", self.logs.get_footer_text(), IconKind::Default);
        if let Some(message) = self.logs.get_footer_message(forward) {
            self.footer.show_info(message, 0);
        }
    }

    fn get_offset(&self) -> Option<Position> {
        if self.logs.content().is_some_and(|c| c.show_timestamps) {
            Some(Position::new(TIMESTAMP_TEXT_LENGTH as u16, 0))
        } else {
            None
        }
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        let response = self.command_palette.process_event(event);
        if response == ResponseEvent::Cancelled {
            self.clear_search();
        } else if response.is_action("palette") {
            self.last_mouse_click = event.position();
            return self.process_event(&self.app_data.get_event(KeyCommand::CommandPaletteOpen));
        } else if response.is_action("timestamps") {
            self.toggle_timestamps();
            return ResponseEvent::Handled;
        } else if response.is_action("copy") {
            self.copy_logs_to_clipboard();
            return ResponseEvent::Handled;
        } else if response.is_action("search") {
            self.search.highlight_position(event.position());
            self.search.show();
            return ResponseEvent::Handled;
        }

        response
    }
}

impl View for LogsView {
    fn process_tick(&mut self) -> ResponseEvent {
        if !self.observer.is_empty() {
            if !self.logs.has_content() {
                let mut content = LogsContent::new(self.app_data.borrow().theme.colors.syntax.logs.clone());
                content.set_timestamps(self.app_data.borrow().config.logs.timestamps.is_none_or(|t| t));
                self.logs.set_content(content);
            }

            let content = self.logs.content_mut().unwrap();
            let mut max_width = 0;

            content.count = 0; // force re-render current logs page
            while let Some(chunk) = self.observer.try_next() {
                for line in chunk.lines {
                    let width = if content.show_timestamps {
                        line.message.chars().count() + TIMESTAMP_TEXT_LENGTH
                    } else {
                        line.message.chars().count()
                    };
                    if max_width < width {
                        max_width = width;
                    }

                    content.lowercase.push(line.message.to_ascii_lowercase());
                    content.lines.push(line);
                }
            }

            if let Some(content) = self.logs.content_mut()
                && content.max_size < max_width
            {
                content.max_size = max_width;
            }

            if self.bound_to_bottom {
                self.logs.scroll_to_end();
            }

            if self.logs.search(self.search.value(), true) {
                self.update_search_count();
            }
        }

        ResponseEvent::Handled
    }

    fn process_disconnection(&mut self) {
        // pass
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled || event.is_mouse(MouseEventKind::LeftClick) {
                return result;
            }
        }

        if self.search.is_visible {
            let result = self.search.process_event(event);
            if self.logs.search(self.search.value(), false) {
                self.logs.scroll_to_current_match(self.get_offset());
                self.update_search_count();
            }

            self.update_bound_to_bottom();
            return result;
        }

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen) {
            self.show_command_palette();
            return ResponseEvent::Handled;
        }

        if let TuiEvent::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::RightClick
        {
            self.show_mouse_menu(mouse.column, mouse.row);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::SearchOpen) {
            self.search.show();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::SearchReset) && !self.search.value().is_empty() {
            self.clear_search();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) {
            return ResponseEvent::Cancelled;
        }

        if self.app_data.has_binding(event, KeyCommand::LogsTimestamps) {
            self.toggle_timestamps();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::ContentCopy) {
            self.copy_logs_to_clipboard();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::MatchNext) && self.logs.matches_count().is_some() {
            self.navigate_match(true);
        }

        if self.app_data.has_binding(event, KeyCommand::MatchPrevious) && self.logs.matches_count().is_some() {
            self.navigate_match(false);
        }

        if let TuiEvent::Key(key) = event
            && (key.code == KeyCode::Down || key.code == KeyCode::End || key.code == KeyCode::PageDown)
            && self.logs.is_at_end()
        {
            self.update_bound_to_bottom();
            self.logs.process_event(event);
        } else if self.logs.process_event(event) == ResponseEvent::Handled {
            self.update_bound_to_bottom();
        }

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.logs.draw(frame, area, self.get_offset());
        self.command_palette.draw(frame, frame.area());
        self.search.draw(frame, frame.area());

        if area.height != self.area.height && self.bound_to_bottom {
            self.area = area;
            self.logs.scroll_to_end();
        }
    }
}

impl Drop for LogsView {
    fn drop(&mut self) {
        self.observer.stop();
    }
}

/// Logs content for [`LogsView`].
struct LogsContent {
    show_timestamps: bool,
    colors: LogsSyntaxColors,
    lines: Vec<LogLine>,
    lowercase: Vec<String>,
    page: Vec<StyledLine>,
    max_size: usize,
    start: usize,
    count: usize,
}

impl LogsContent {
    /// Returns new [`LogsContent`] instance.
    fn new(colors: LogsSyntaxColors) -> Self {
        Self {
            show_timestamps: true,
            colors,
            lines: Vec::with_capacity(INITIAL_LOGS_VEC_SIZE),
            lowercase: Vec::with_capacity(INITIAL_LOGS_VEC_SIZE),
            page: Vec::default(),
            max_size: 0,
            start: 0,
            count: 0,
        }
    }

    fn set_timestamps(&mut self, enabled: bool) {
        if self.show_timestamps != enabled {
            self.show_timestamps = enabled;
            self.count = 0;
        }
    }

    fn toggle_timestamps(&mut self) {
        self.show_timestamps = !self.show_timestamps;
        self.count = 0;
    }

    fn style_log_line(&self, line: &LogLine) -> Vec<(Style, String)> {
        let log_colors = if line.is_error {
            &self.colors.error
        } else {
            &self.colors.string
        };

        if self.show_timestamps {
            vec![
                (
                    (&self.colors.timestamp).into(),
                    line.datetime.format(TIMESTAMP_TEXT_FORMAT).to_string(),
                ),
                (log_colors.into(), line.message.clone()),
            ]
        } else {
            vec![(log_colors.into(), line.message.clone())]
        }
    }
}

impl Content for LogsContent {
    fn page(&mut self, start: usize, count: usize) -> &[StyledLine] {
        if start >= self.lines.len() {
            return &[];
        }

        let end = start + count;
        let end = if end >= self.lines.len() { self.lines.len() } else { end };
        if self.start != start || self.count != count {
            self.start = start;
            self.count = count;
            self.page = Vec::with_capacity(end - start);

            for line in &self.lines[start..end] {
                self.page.push(self.style_log_line(line));
            }
        }

        &self.page
    }

    fn len(&self) -> usize {
        self.lines.len()
    }

    fn hash(&self) -> u64 {
        0
    }

    fn to_plain_text(&self, range: Option<Selection>) -> String {
        let range = range.map(|r| r.sorted());
        let (start, end) = range.map_or_else(|| (0, self.lines.len()), |(s, e)| (s.y, e.y));
        let start_line = start.min(self.lines.len().saturating_sub(1));
        let end_line = end.min(self.lines.len().saturating_sub(1));
        let (start, end) = range.map_or_else(|| (0, self.line_size(end_line).saturating_sub(1)), |(s, e)| (s.x, e.x));

        let mut result = String::new();
        for i in start_line..=end_line {
            let line = &self.lines[i];
            if i == start_line || i == end_line {
                let text = if self.show_timestamps {
                    format!("{}{}", line.datetime.format(TIMESTAMP_TEXT_FORMAT), line.message)
                } else {
                    line.message.clone()
                };

                if i == start_line && i == end_line {
                    result.push_str(substring(&text, start, (end + 1).saturating_sub(start)));
                    if text.chars().count() < end + 1 {
                        result.push('\n');
                    }
                } else if i == start_line {
                    result.push_str(slice_from(&text, start));
                    result.push('\n');
                } else if i == end_line {
                    result.push_str(slice_to(&text, end + 1));
                    if text.chars().count() < end + 1 {
                        result.push('\n');
                    }
                }
            } else {
                if self.show_timestamps {
                    result.push_str(&line.datetime.format(TIMESTAMP_TEXT_FORMAT).to_string());
                }

                result.push_str(&line.message);
                result.push('\n');
            }
        }

        result
    }

    fn search_first(&self, pattern: &str) -> Option<MatchPosition> {
        let pattern = pattern.to_ascii_lowercase();
        for (y, line) in self.lowercase.iter().enumerate() {
            if let Some(x) = line.find(&pattern) {
                return Some(MatchPosition::new(x, y, pattern.len()));
            }
        }

        None
    }

    fn search(&self, pattern: &str) -> Vec<MatchPosition> {
        let pattern = pattern.to_ascii_lowercase();
        let mut matches = Vec::new();
        for (y, line) in self.lowercase.iter().enumerate() {
            for (x, _) in line.match_indices(&pattern) {
                matches.push(MatchPosition::new(x, y, pattern.len()));
            }
        }

        matches
    }

    fn max_size(&self) -> usize {
        self.max_size
    }

    fn line_size(&self, line_no: usize) -> usize {
        let size = self.lines.get(line_no).map(|l| l.message.chars().count()).unwrap_or_default();
        if self.show_timestamps {
            size + TIMESTAMP_TEXT_LENGTH
        } else {
            size
        }
    }

    fn word_bounds(&self, position: ContentPosition) -> Option<(usize, usize)> {
        if let Some(line) = self.lines.get(position.y) {
            if self.show_timestamps {
                let idx = position.x.saturating_sub(TIMESTAMP_TEXT_LENGTH);
                let bounds = b4n_common::word_bounds(&line.message, idx);
                bounds.map(|(x, y)| (x + TIMESTAMP_TEXT_LENGTH, y + TIMESTAMP_TEXT_LENGTH))
            } else {
                b4n_common::word_bounds(&line.message, position.x)
            }
        } else {
            None
        }
    }
}
