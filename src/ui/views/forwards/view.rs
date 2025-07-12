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
        views::{ListHeader, ListViewer, PortForwardsList, View, get_breadcrumbs_namespace},
        widgets::{ActionItem, ActionsListBuilder, Button, CommandPalette, Dialog, Filter, FooterMessage},
    },
};

pub const VIEW_NAME: &str = "port forwards";

/// Port forwards view.
pub struct ForwardsView {
    pub header: ListHeader,
    pub list: ListViewer<PortForwardsList>,
    app_data: SharedAppData,
    namespace: Namespace,
    worker: SharedBgWorker,
    command_palette: CommandPalette,
    filter: Filter,
    modal: Dialog,
    footer_tx: UnboundedSender<FooterMessage>,
    is_closing: bool,
}

impl ForwardsView {
    /// Creates new [`ForwardsView`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: UnboundedSender<FooterMessage>) -> Self {
        let namespace: Namespace = get_breadcrumbs_namespace(&app_data.borrow().current, VIEW_NAME).into();
        let view = if namespace.is_all() {
            ViewType::Full
        } else {
            ViewType::Compact
        };
        let filter = Filter::new(Rc::clone(&app_data), Some(Rc::clone(&worker)), 60);
        let mut list = ListViewer::new(Rc::clone(&app_data), PortForwardsList::default(), view);
        list.table.update(worker.borrow_mut().get_port_forwards_list(&namespace));

        Self {
            header: ListHeader::new(Rc::clone(&app_data), Some(VIEW_NAME), list.table.len()),
            list,
            app_data,
            namespace,
            worker,
            command_palette: CommandPalette::default(),
            filter,
            modal: Dialog::default(),
            footer_tx,
            is_closing: false,
        }
    }

    fn process_command_palette_events(&mut self, key: crossterm::event::KeyEvent) -> bool {
        if key.code == KeyCode::Char(':') || key.code == KeyCode::Char('>') {
            let builder = ActionsListBuilder::from_kinds(self.app_data.borrow().kinds.as_deref())
                .with_close()
                .with_quit()
                .with_action(
                    ActionItem::new("stop")
                        .with_description("stops selected port forwarding rules")
                        .with_response(ResponseEvent::Action("stop_selected")),
                );
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
            self.command_palette.show();
            true
        } else {
            false
        }
    }

    /// Sets filter on the port forwards list.
    pub fn set_filter(&mut self) {
        let value = self.filter.value();
        self.header.show_filtered_icon(!value.is_empty());
        if value.is_empty() {
            if self.list.table.is_filtered() {
                self.list.table.filter(None);
                self.header.set_count(self.list.table.len());
            }
        } else if !self.list.table.is_filtered() || self.list.table.get_filter().is_some_and(|f| f != value) {
            self.list.table.filter(Some(value.to_owned()));
            self.header.set_count(self.list.table.len());
        }
    }

    /// Shows dialog to stop port forwarding rules if anything is selected.
    fn ask_stop_port_forwards(&mut self) {
        if self.list.table.is_anything_selected() {
            self.modal = self.new_stop_dialog();
            self.modal.show();
        }
    }

    /// Stops selected port forwarding rules.
    fn stop_selected_port_forwards(&mut self) {
        self.worker
            .borrow_mut()
            .stop_port_forwards(&self.list.table.table.list.get_selected_uids());
        self.list.table.table.list.deselect_all();

        self.footer_tx
            .send(FooterMessage::info(
                " Selected port forwarding rules have been stopped…",
                2_000,
            ))
            .unwrap();
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
    fn displayed_namespace(&self) -> &str {
        self.namespace.as_str()
    }

    fn is_namespaces_selector_allowed(&self) -> bool {
        true
    }

    fn is_resources_selector_allowed(&self) -> bool {
        true
    }

    fn handle_resources_selector_event(&mut self, event: &ResponseEvent) {
        if matches!(event, ResponseEvent::ChangeKind(_)) {
            self.is_closing = true;
        }
    }

    fn handle_namespace_change(&mut self) {
        self.namespace = get_breadcrumbs_namespace(&self.app_data.borrow().current, VIEW_NAME).into();
        self.list.view = if self.namespace.is_all() {
            ViewType::Full
        } else {
            ViewType::Compact
        };
        self.list
            .table
            .update(self.worker.borrow_mut().get_port_forwards_list(&self.namespace));
        self.header.set_count(self.list.table.len());
    }

    fn process_tick(&mut self) -> ResponseEvent {
        if self.is_closing {
            return ResponseEvent::Cancelled;
        }

        let mut worker = self.worker.borrow_mut();
        if worker.is_port_forward_list_changed() {
            self.list.table.update(worker.get_port_forwards_list(&self.namespace));
            self.header.set_count(self.list.table.len());
        }

        ResponseEvent::Handled
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if self.filter.is_visible {
            self.filter.process_key(key);
            self.set_filter();
            return ResponseEvent::Handled;
        }

        if self.modal.is_visible {
            if self.modal.process_key(key) == ResponseEvent::DeleteResources {
                self.stop_selected_port_forwards();
            }

            return ResponseEvent::Handled;
        }

        if self.command_palette.is_visible {
            return match self.command_palette.process_key(key) {
                ResponseEvent::ChangeKind(kind) => {
                    self.is_closing = true;
                    ResponseEvent::ChangeKind(kind)
                },
                ResponseEvent::Action("stop_selected") => {
                    self.ask_stop_port_forwards();
                    ResponseEvent::Handled
                },
                response_event => response_event,
            };
        }

        if self.process_command_palette_events(key) {
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Esc && !self.filter.value().is_empty() {
            self.filter.reset();
            self.set_filter();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Esc || (key.code == KeyCode::Char('f') && key.modifiers == KeyModifiers::CONTROL) {
            return ResponseEvent::Cancelled;
        }

        if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
            self.ask_stop_port_forwards();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('/') {
            self.filter.show();
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
        self.filter.draw(frame, frame.area());
    }
}
