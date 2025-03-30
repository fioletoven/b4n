use std::rc::Rc;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::{SharedAppData, SharedBgWorker},
    kubernetes::Namespace,
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        views::{View, content::ContentViewer},
        widgets::{ActionsListBuilder, CommandPalette, FooterMessage},
    },
};

/// Logs view.
pub struct LogsView {
    pub logs: ContentViewer,
    app_data: SharedAppData,
    worker: SharedBgWorker,
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
        pod_container: String,
        footer_tx: UnboundedSender<FooterMessage>,
    ) -> Self {
        let logs = ContentViewer::new(Rc::clone(&app_data)).with_header(
            " logs  ",
            pod_namespace,
            "pods".to_owned(),
            pod_name,
            Some(pod_container),
        );

        Self {
            logs,
            app_data,
            worker,
            command_palette: CommandPalette::default(),
            footer_tx,
        }
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

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.logs.draw(frame, area);
        self.command_palette.draw(frame, frame.area());
    }
}
