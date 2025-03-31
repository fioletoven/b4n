use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
};
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::{SharedAppData, SharedBgWorker},
    kubernetes::{Namespace, resources::PODS},
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        views::{View, content::ContentViewer},
        widgets::{ActionsListBuilder, CommandPalette, FooterMessage},
    },
};

use super::{LogsObserver, LogsObserverError, PodRef};

/// Logs view.
pub struct LogsView {
    pub logs: ContentViewer,
    app_data: SharedAppData,
    worker: SharedBgWorker,
    observer: LogsObserver,
    command_palette: CommandPalette,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl LogsView {
    /// Creates new [`LogsView`] instance.
    pub fn new(
        app_data: SharedAppData,
        worker: SharedBgWorker,
        pod_name: String,
        pod_namespace: Namespace,
        pod_container: Option<String>,
        footer_tx: UnboundedSender<FooterMessage>,
    ) -> Result<Self, LogsObserverError> {
        let mut observer = LogsObserver::new();
        if let Some(client) = worker.borrow().kubernetes_client() {
            let pod = PodRef {
                name: pod_name.clone(),
                namespace: pod_namespace.clone(),
                container: pod_container.clone(),
            };
            observer.start(client, pod)?;
        }

        let logs = ContentViewer::new(Rc::clone(&app_data)).with_header(
            " logs  ",
            pod_namespace,
            PODS.to_owned(),
            pod_name,
            pod_container,
        );

        Ok(Self {
            logs,
            app_data,
            worker,
            observer,
            command_palette: CommandPalette::default(),
            footer_tx,
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
    fn process_event_ticks(&mut self) {
        while let Some(log) = self.observer.try_next() {
            let mut content = self.logs.take_content().unwrap_or_default();
            let mut max_width = 0;

            for line in log.lines {
                let width = line.chars().count();
                if max_width < width {
                    max_width = width;
                }

                content.push(vec![(Style::new().fg(Color::Black), line)]);
            }

            self.logs.set_content(content, max_width);
        }
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

        self.logs.process_key(key)
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
