use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{Frame, layout::Rect};
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    core::{SharedAppData, SharedBgWorker},
    ui::{
        ResponseEvent, Responsive, TuiEvent,
        views::View,
        widgets::{ActionsListBuilder, CommandPalette, FooterMessage},
    },
};

/// Port forwards view.
pub struct ForwardsView {
    app_data: SharedAppData,
    worker: SharedBgWorker,
    command_palette: CommandPalette,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl ForwardsView {
    /// Creates new [`ForwardsView`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: UnboundedSender<FooterMessage>) -> Self {
        Self {
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

impl View for ForwardsView {
    fn process_disconnection(&mut self) {
        self.command_palette.hide();
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

        ResponseEvent::Handled
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.command_palette.draw(frame, frame.area());
    }
}
