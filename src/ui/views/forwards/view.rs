use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    core::{SharedAppData, SharedBgWorker},
    kubernetes::Namespace,
    ui::{
        ResponseEvent, Responsive, Table, TuiEvent, ViewType,
        views::{ListPane, PortForwardsList, View, forwards::HeaderPane, utils},
        widgets::{ActionsListBuilder, Button, CommandPalette, Dialog, FooterMessage},
    },
};

pub const VIEW_NAME: &str = "port forwards";

/// Port forwards view.
pub struct ForwardsView {
    pub header: HeaderPane,
    pub list: ListPane<PortForwardsList>,
    app_data: SharedAppData,
    namespace: Namespace,
    worker: SharedBgWorker,
    command_palette: CommandPalette,
    modal: Dialog,
    footer_tx: UnboundedSender<FooterMessage>,
}

impl ForwardsView {
    /// Creates new [`ForwardsView`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let namespace = utils::get_breadcumbs_namespace(&app_data.borrow().current, VIEW_NAME).into();
        let view = if app_data.borrow().current.namespace.is_all() {
            ViewType::Full
        } else {
            ViewType::Compact
        };
        let mut list = ListPane::new(Rc::clone(&app_data), PortForwardsList::default(), view);
        list.table.update(worker.borrow_mut().get_port_forwards_list(&namespace));

        Self {
            header: HeaderPane::new(Rc::clone(&app_data), list.table.len()),
            list,
            app_data,
            namespace,
            worker,
            command_palette: CommandPalette::default(),
            modal: Dialog::default(),
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

    /// Shows stop port forwards dialog if anything is selected.
    fn ask_stop_port_forwards(&mut self) {
        if self.list.table.is_anything_selected() {
            self.modal = self.new_stop_dialog();
            self.modal.show();
        }
    }

    /// Stops selected port forwards.
    fn stop_selected_port_forwards(&mut self) {
        self.worker
            .borrow_mut()
            .stop_port_forwards(&self.list.table.table.list.get_selected_uids());
        self.list.table.table.list.deselect_all();
    }

    /// Creates new stop dialog.
    fn new_stop_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "Are you sure you want to stop the selected port forwarding rules?".to_owned(),
            vec![
                Button::new(
                    "Stop".to_owned(),
                    ResponseEvent::DeleteResources,
                    colors.modal.btn_delete.clone(),
                ),
                Button::new("Cancel".to_owned(), ResponseEvent::Cancelled, colors.modal.btn_cancel.clone()),
            ],
            60,
            colors.modal.text,
        )
    }
}

impl View for ForwardsView {
    fn is_namespaces_selector_allowed(&self) -> bool {
        true
    }

    fn displayed_namespace(&self) -> &str {
        self.namespace.as_str()
    }

    fn process_namespace_change(&mut self) {
        self.namespace = utils::get_breadcumbs_namespace(&self.app_data.borrow().current, VIEW_NAME).into();
        self.list
            .table
            .update(self.worker.borrow_mut().get_port_forwards_list(&self.namespace));
        self.header.set_count(self.list.table.len());
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_tick(&mut self) -> ResponseEvent {
        let mut worker = self.worker.borrow_mut();
        if worker.is_port_forward_list_changed() {
            self.list.table.update(worker.get_port_forwards_list(&self.namespace));
            self.header.set_count(self.list.table.len());
        }

        ResponseEvent::Handled
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if self.modal.is_visible {
            if self.modal.process_key(key) == ResponseEvent::DeleteResources {
                self.stop_selected_port_forwards();
            }

            return ResponseEvent::Handled;
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

        if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
            self.ask_stop_port_forwards();
            return ResponseEvent::Handled;
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

        self.modal.draw(frame, frame.area());
        self.command_palette.draw(frame, frame.area());
    }
}
