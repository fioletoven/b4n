use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;
use time::{format_description::BorrowedFormatItem, macros::format_description};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::SharedAppData,
    kubernetes::{Namespace, client::KubernetesClient, resources::PODS},
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        views::{View, content::ContentViewer},
        widgets::{ActionsListBuilder, CommandPalette, FooterMessage},
    },
};

use super::{LogsObserver, LogsObserverError, PodRef};

const DATETIME_FORMAT: &[BorrowedFormatItem<'_>] = format_description!(
    version = 2,
    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6] "
);

/// Logs view.
pub struct LogsView {
    pub logs: ContentViewer,
    app_data: SharedAppData,
    observer: LogsObserver,
    command_palette: CommandPalette,
    footer_tx: UnboundedSender<FooterMessage>,
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
        footer_tx: UnboundedSender<FooterMessage>,
    ) -> Result<Self, LogsObserverError> {
        let mut observer = LogsObserver::new();
        let pod = PodRef {
            name: pod_name.clone(),
            namespace: pod_namespace.clone(),
            container: pod_container.clone(),
        };
        observer.start(client, pod)?;

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
            observer,
            command_palette: CommandPalette::default(),
            footer_tx,
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
            let colors = &self.app_data.borrow().theme.colors;
            if !self.logs.has_content() {
                self.logs.set_content(Vec::with_capacity(200), 0);
            }

            let content = self.logs.content_mut().unwrap();
            let mut max_width = 0;

            while let Some(chunk) = self.observer.try_next() {
                for line in chunk.lines {
                    let width = line.message.chars().count();
                    if max_width < width {
                        max_width = width;
                    }

                    content.push(vec![
                        (
                            (&colors.syntax.logs.timestamp).into(),
                            line.datetime.format(DATETIME_FORMAT).unwrap(),
                        ),
                        ((&colors.syntax.logs.string).into(), line.message),
                    ]);
                }
            }

            self.logs.update_content_width(max_width);
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

        if self.logs.process_key(key) == ResponseEvent::Handled {
            self.bound_to_bottom = false;
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
