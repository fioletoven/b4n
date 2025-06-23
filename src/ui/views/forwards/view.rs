use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    core::{SharedAppData, SharedBgWorker},
    ui::{
        ResponseEvent, Responsive, Table, TuiEvent, ViewType,
        views::{ListPane, PortForwardsList, View, forwards::HeaderPane},
        widgets::{ActionsListBuilder, CommandPalette, FooterMessage},
    },
};

/// Port forwards view.
pub struct ForwardsView {
    pub header: HeaderPane,
    pub list: ListPane<PortForwardsList>,
    app_data: SharedAppData,
    worker: SharedBgWorker,
    command_palette: CommandPalette,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl ForwardsView {
    /// Creates new [`ForwardsView`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let view = if app_data.borrow().current.namespace.is_all() {
            ViewType::Full
        } else {
            ViewType::Compact
        };
        let mut list = ListPane::new(Rc::clone(&app_data), PortForwardsList::default(), view);
        list.table.update(worker.borrow_mut().get_port_forwards_list());

        Self {
            header: HeaderPane::new(Rc::clone(&app_data), list.table.len()),
            list,
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

    fn process_tick(&mut self) -> ResponseEvent {
        let mut worker = self.worker.borrow_mut();
        if worker.is_port_forward_list_changed() {
            self.list.table.update(worker.get_port_forwards_list());
            self.header.set_count(self.list.table.len());
        }

        ResponseEvent::Handled
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

        self.list.process_key(key)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        self.header.draw(frame, layout[0]);
        self.list.draw(frame, layout[1]);

        self.command_palette.draw(frame, frame.area());
    }
}
