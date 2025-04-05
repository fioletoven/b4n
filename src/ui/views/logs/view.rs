use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect, style::Style};
use std::rc::Rc;

use crate::{
    app::SharedAppData,
    kubernetes::{Namespace, client::KubernetesClient, resources::PODS},
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        theme::LogsSyntaxColors,
        views::{
            View,
            content::{Content, ContentViewer, StyledLine},
        },
        widgets::{ActionsListBuilder, CommandPalette},
    },
};

use super::{LogLine, LogsObserver, LogsObserverError, PodRef};

const INITIAL_LOGS_VEC_SIZE: usize = 5_000;

/// Logs view.
pub struct LogsView {
    logs: ContentViewer<LogsContent>,
    app_data: SharedAppData,
    observer: LogsObserver,
    command_palette: CommandPalette,
    bound_to_bottom: bool,
}

impl LogsView {
    /// Creates new [`LogsView`] instance.
    pub fn new(
        app_data: SharedAppData,
        client: &KubernetesClient,
        pod_name: String,
        pod_namespace: Namespace,
        pod_container: Option<String>,
        previous: bool,
    ) -> Result<Self, LogsObserverError> {
        let pod = PodRef {
            name: pod_name.clone(),
            namespace: pod_namespace.clone(),
            container: pod_container.clone(),
        };
        let logs = ContentViewer::new(Rc::clone(&app_data)).with_header(
            if previous { "previous logs" } else { "logs" },
            '',
            pod_namespace,
            PODS.to_owned(),
            pod_name,
            pod_container,
        );

        let mut observer = LogsObserver::new();
        observer.start(client, pod, app_data.borrow().config.logs.lines, previous)?;

        Ok(Self {
            logs,
            app_data,
            observer,
            command_palette: CommandPalette::default(),
            bound_to_bottom: true,
        })
    }

    fn process_command_palette_events(&mut self, key: crossterm::event::KeyEvent) -> bool {
        if key.code == KeyCode::Char(':') || key.code == KeyCode::Char('>') {
            let builder = ActionsListBuilder::default().with_close().with_quit();
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
            self.command_palette.show();
            true
        } else {
            false
        }
    }
}

impl View for LogsView {
    fn process_tick(&mut self) {
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
                    let width = line.message.chars().count() + 24;
                    if max_width < width {
                        max_width = width;
                    }

                    content.lines.push(line);
                }
            }

            self.logs.update_max_width(max_width);
            if self.bound_to_bottom {
                self.logs.scroll_to_end();
            }
        }
    }

    fn process_disconnection(&mut self) {
        // pass
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if self.command_palette.is_visible {
            return self.command_palette.process_key(key);
        }

        if self.process_command_palette_events(key) {
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Esc {
            return ResponseEvent::Cancelled;
        }

        if (key.code == KeyCode::Down || key.code == KeyCode::End || key.code == KeyCode::PageDown) && self.logs.is_at_end() {
            self.bound_to_bottom = true;
            self.logs.header.set_icon('');
            self.logs.process_key(key);
        } else if self.logs.process_key(key) == ResponseEvent::Handled {
            self.bound_to_bottom = false;
            self.logs.header.set_icon('');
        }

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.logs.draw(frame, area);
        self.command_palette.draw(frame, frame.area());
    }
}

impl Drop for LogsView {
    fn drop(&mut self) {
        self.observer.stop();
    }
}

/// Logs content for [`LogsView`].
struct LogsContent {
    colors: LogsSyntaxColors,
    lines: Vec<LogLine>,
    page: Vec<StyledLine>,
    start: usize,
    count: usize,
}

impl LogsContent {
    /// Returns new [`LogsContent`] instance.
    fn new(colors: LogsSyntaxColors) -> Self {
        Self {
            colors,
            lines: Vec::with_capacity(INITIAL_LOGS_VEC_SIZE),
            page: Vec::default(),
            start: 0,
            count: 0,
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
                self.page.push(style_log_line(line, &self.colors));
            }
        }

        &self.page
    }

    fn len(&self) -> usize {
        self.lines.len()
    }
}

fn style_log_line(line: &LogLine, colors: &LogsSyntaxColors) -> Vec<(Style, String)> {
    let log_colors = if line.is_error { &colors.error } else { &colors.string };
    vec![
        (
            (&colors.timestamp).into(),
            line.datetime.format("%Y-%m-%d %H:%M:%S%.3f ").to_string(),
        ),
        (log_colors.into(), line.message.clone()),
    ]
}
