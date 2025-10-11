use kube::discovery::Scope;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::rc::Rc;

use crate::{
    core::{SharedAppData, SharedAppDataExt, SharedBgWorker},
    kubernetes::Namespace,
    ui::{
        KeyCommand, MouseEventKind, ResponseEvent, Responsive, Table, TuiEvent, ViewType,
        viewers::{ListHeader, ListViewer},
        views::{PortForwardsList, View},
        widgets::{ActionItem, ActionsListBuilder, Button, CommandPalette, Dialog, Filter, FooterTx},
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
    footer_tx: FooterTx,
    is_closing: bool,
}

impl ForwardsView {
    /// Creates new [`ForwardsView`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: FooterTx) -> Self {
        let (namespace, view) = get_current_namespace(&app_data);
        let filter = Filter::new(Rc::clone(&app_data), Some(Rc::clone(&worker)), 60);
        let mut list = ListViewer::new(Rc::clone(&app_data), PortForwardsList::default(), view);
        list.table.update(worker.borrow_mut().get_port_forwards_list(&namespace));
        let header = ListHeader::new(Rc::clone(&app_data), list.table.len())
            .with_kind(VIEW_NAME)
            .with_namespace(namespace.as_str())
            .with_scope(Scope::Namespaced);

        Self {
            header,
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

    /// Shows command palette.
    fn show_command_palette(&mut self) {
        let builder = ActionsListBuilder::from_kinds(self.app_data.borrow().kinds.as_deref(), false)
            .with_close()
            .with_quit()
            .with_action(
                ActionItem::new("stop")
                    .with_description("stops selected port forwarding rules")
                    .with_response(ResponseEvent::Action("stop_selected")),
            );
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
        self.command_palette.show();
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
            .show_info(" Selected port forwarding rules have been stopped…", 2_000);
    }

    /// Creates new stop dialog.
    fn new_stop_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "Are you sure you want to stop the selected port forwarding rules?".to_owned(),
            vec![
                Button::new("Stop", ResponseEvent::DeleteResources, &colors.modal.btn_delete),
                Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
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
        !self.filter.is_visible && !self.modal.is_visible && !self.command_palette.is_visible
    }

    fn is_resources_selector_allowed(&self) -> bool {
        !self.filter.is_visible && !self.modal.is_visible && !self.command_palette.is_visible
    }

    fn handle_resources_selector_event(&mut self, event: &ResponseEvent) {
        if matches!(event, ResponseEvent::ChangeKind(_)) {
            self.is_closing = true;
        }
    }

    fn handle_namespace_change(&mut self) {
        (self.namespace, self.list.view) = get_current_namespace(&self.app_data);
        self.list
            .table
            .update(self.worker.borrow_mut().get_port_forwards_list(&self.namespace));
        self.header.set_count(self.list.table.len());
        self.header.set_namespace(self.namespace.as_option());
    }

    fn process_tick(&mut self) -> ResponseEvent {
        if self.is_closing {
            return ResponseEvent::Cancelled;
        }

        let mut worker = self.worker.borrow_mut();
        if worker.check_port_forward_list_changed() {
            self.list.table.update(worker.get_port_forwards_list(&self.namespace));
            self.header.set_count(self.list.table.len());
        }

        ResponseEvent::Handled
    }

    fn process_disconnection(&mut self) {
        self.command_palette.hide();
    }

    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if self.filter.is_visible {
            self.filter.process_event(event);
            self.set_filter();
            return ResponseEvent::Handled;
        }

        if self.modal.is_visible {
            if self.modal.process_event(event) == ResponseEvent::DeleteResources {
                self.stop_selected_port_forwards();
            }

            return ResponseEvent::Handled;
        }

        if self.command_palette.is_visible {
            return match self.command_palette.process_event(event) {
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

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen)
            || event.is_in(MouseEventKind::RightClick, self.list.area)
        {
            self.show_command_palette();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::FilterReset) && !self.filter.value().is_empty() {
            self.filter.reset();
            self.set_filter();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateBack)
            || self.app_data.has_binding(event, KeyCommand::PortForwardsOpen)
        {
            return ResponseEvent::Cancelled;
        }

        if self.app_data.has_binding(event, KeyCommand::NavigateDelete) {
            self.ask_stop_port_forwards();
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::FilterOpen) {
            self.filter.show();
            return ResponseEvent::Handled;
        }

        self.list.process_event(event)
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

fn get_current_namespace(app_data: &SharedAppData) -> (Namespace, ViewType) {
    let namespace = app_data.borrow().current.get_namespace();
    let view = if namespace.is_all() {
        ViewType::Full
    } else {
        ViewType::Compact
    };

    (namespace, view)
}
