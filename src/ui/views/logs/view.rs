use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Rect, Size},
    style::Style,
};
use std::rc::Rc;

use crate::{
    core::{SharedAppData, SharedAppDataExt, SharedBgWorker},
    kubernetes::{PodRef, ResourceRef, client::KubernetesClient, resources::PODS},
    ui::{
        KeyCombination, KeyCommand, ResponseEvent, Responsive, TuiEvent,
        theme::LogsSyntaxColors,
        views::{
            View,
            content::{Content, ContentViewer, StyledLine},
            content_search::MatchPosition,
        },
        widgets::{ActionItem, ActionsListBuilder, CommandPalette, FooterTx, IconKind, Search},
    },
};

use super::{LogLine, LogsObserver, LogsObserverError};

const INITIAL_LOGS_VEC_SIZE: usize = 5_000;
const TIMESTAMP_TEXT_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f ";
const TIMESTAMP_TEXT_LENGTH: usize = 24;

/// Logs view.
pub struct LogsView {
    logs: ContentViewer<LogsContent>,
    app_data: SharedAppData,
    observer: LogsObserver,
    command_palette: CommandPalette,
    search: Search,
    footer: FooterTx,
    bound_to_bottom: bool,
}

impl LogsView {
    /// Creates new [`LogsView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        client: &KubernetesClient,
        resource: ResourceRef,
        previous: bool,
        footer: FooterTx,
    ) -> Result<Self, LogsObserverError> {
        let pod = PodRef {
            name: resource.name.clone().unwrap_or_default(),
            namespace: resource.namespace.clone(),
            container: resource.container.clone(),
        };
        let color = app_data.borrow().theme.colors.syntax.logs.search;
        let logs = ContentViewer::new(Rc::clone(&app_data), color).with_header(
            if previous { "previous logs" } else { "logs" },
            '',
            resource.namespace,
            PODS.into(),
            resource.name.unwrap_or_default(),
            resource.container,
        );

        let search = Search::new(Rc::clone(&app_data), Some(worker), 60);
        let mut observer = LogsObserver::new();
        observer.start(client, pod, app_data.borrow().config.logs.lines, previous);

        Ok(Self {
            logs,
            app_data,
            observer,
            command_palette: CommandPalette::default(),
            search,
            footer,
            bound_to_bottom: true,
        })
    }

    fn process_command_palette_events(&mut self, key: KeyCombination) -> bool {
        if self.app_data.has_binding(&key, KeyCommand::CommandPaletteOpen) {
            let builder = ActionsListBuilder::default()
                .with_close()
                .with_quit()
                .with_action(
                    ActionItem::new("timestamps")
                        .with_description("toggles the display of timestamps")
                        .with_response(ResponseEvent::Action("timestamps")),
                )
                .with_action(
                    ActionItem::new("copy")
                        .with_description("copies logs to the clipboard")
                        .with_response(ResponseEvent::Action("copy")),
                );
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
            self.command_palette.show();
            true
        } else {
            false
        }
    }

    fn toggle_timestamps(&mut self) {
        if let Some(content) = self.logs.content_mut() {
            content.toggle_timestamps();
            if content.show_timestamps {
                self.logs.max_width_add(TIMESTAMP_TEXT_LENGTH);
            } else {
                self.logs.max_width_sub(TIMESTAMP_TEXT_LENGTH);
            }

            self.logs.reset_horizontal_scroll();
        }
    }

    fn copy_logs_to_clipboard(&self) {
        if self.logs.content().is_some() {
            let result: Result<ClipboardContext, _> = ClipboardProvider::new();
            if let Ok(mut ctx) = result
                && ctx.set_contents(self.get_logs_as_string()).is_ok()
            {
                self.footer.show_info(" container logs copied to clipboard…", 1_500);
            }
        }
    }

    fn get_logs_as_string(&self) -> String {
        if let Some(content) = self.logs.content() {
            let mut result = String::new();
            for line in &content.lines {
                if content.show_timestamps {
                    result.push_str(&line.datetime.format(TIMESTAMP_TEXT_FORMAT).to_string());
                    result.push(' ');
                }

                result.push_str(&line.message);
                result.push('\n');
            }

            result
        } else {
            String::default()
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
            .set_text("logs_search", self.logs.get_footer_text(), IconKind::Default);
        self.search.set_matches(self.logs.matches_count());
    }

    fn navigate_match(&mut self, forward: bool) {
        self.logs.navigate_match(forward, self.get_offset());
        self.footer
            .set_text("logs_search", self.logs.get_footer_text(), IconKind::Default);
        if let Some(message) = self.logs.get_footer_message(forward) {
            self.footer.show_info(message, 0);
        }
    }

    fn get_offset(&self) -> Option<Size> {
        if self.logs.content().is_some_and(|c| c.show_timestamps) {
            Some(Size::new(TIMESTAMP_TEXT_LENGTH as u16, 0))
        } else {
            None
        }
    }
}

impl View for LogsView {
    fn process_tick(&mut self) -> ResponseEvent {
        if !self.observer.is_empty() {
            if !self.logs.has_content() {
                self.logs
                    .set_content(LogsContent::new(self.app_data.borrow().theme.colors.syntax.logs.clone()), 0);
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

            self.logs.update_max_width(max_width);
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

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if self.app_data.has_binding(&key, KeyCommand::ApplicationExit) {
            return ResponseEvent::ExitApplication;
        }

        if self.command_palette.is_visible {
            let response = self.command_palette.process_key(key);
            if response == ResponseEvent::Cancelled {
                self.clear_search();
            } else if response.is_action("timestamps") {
                self.toggle_timestamps();
                return ResponseEvent::Handled;
            }

            return response;
        }

        if self.search.is_visible {
            let result = self.search.process_key(key);
            if self.logs.search(self.search.value(), false) {
                self.logs.scroll_to_current_match(self.get_offset());
                self.update_search_count();
            }

            self.update_bound_to_bottom();
            return result;
        }

        if self.process_command_palette_events(key) {
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(&key, KeyCommand::SearchOpen) {
            self.search.show();
        }

        if self.app_data.has_binding(&key, KeyCommand::SearchReset) && !self.search.value().is_empty() {
            self.clear_search();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(&key, KeyCommand::NavigateBack) {
            return ResponseEvent::Cancelled;
        }

        if self.app_data.has_binding(&key, KeyCommand::LogsTimestamps) {
            self.toggle_timestamps();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(&key, KeyCommand::ContentCopy) {
            self.copy_logs_to_clipboard();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(&key, KeyCommand::MatchNext) && self.logs.matches_count().is_some() {
            self.navigate_match(true);
        }

        if self.app_data.has_binding(&key, KeyCommand::MatchPrevious) && self.logs.matches_count().is_some() {
            self.navigate_match(false);
        }

        if (key.code == KeyCode::Down || key.code == KeyCode::End || key.code == KeyCode::PageDown) && self.logs.is_at_end() {
            self.update_bound_to_bottom();
            self.logs.process_key(key);
        } else if self.logs.process_key(key) == ResponseEvent::Handled {
            self.update_bound_to_bottom();
        }

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.logs.draw(frame, area, self.get_offset());
        self.command_palette.draw(frame, frame.area());
        self.search.draw(frame, frame.area());
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
            start: 0,
            count: 0,
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
}
