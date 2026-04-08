use b4n_common::NotificationSink;
use b4n_config::keys::KeyCommand;
use b4n_kube::Namespace;
use b4n_tui::widgets::{ActionItem, ActionsListBuilder, Button, Dialog};
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, TuiEvent, table::Table, table::ViewType};
use kube::discovery::Scope;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use std::rc::Rc;

use crate::core::{SharedAppData, SharedAppDataExt, SharedBgWorker};
use crate::kube::kinds::ActionsListBuilderKindExt;
use crate::ui::presentation::{ListHeader, ListViewer};
use crate::ui::views::{PortForwardsList, View};
use crate::ui::widgets::{CommandPalette, Filter};

pub const VIEW_NAME: &str = "port forwards";

/// Port forwards view.
pub struct ForwardsView {
    pub header: ListHeader,
    pub list: ListViewer<PortForwardsList>,
    app_data: SharedAppData,
    namespace: Namespace,
    worker: SharedBgWorker,
    last_mouse_click: Option<Position>,
    modal: Dialog,
    command_palette: CommandPalette,
    filter: Filter,
    footer_tx: NotificationSink,
    is_closing: bool,
}

impl ForwardsView {
    /// Creates new [`ForwardsView`] instance.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker, footer_tx: NotificationSink) -> Self {
        let (namespace, view) = get_current_namespace(&app_data);
        let filter = Filter::new(Rc::clone(&app_data), Some(Rc::clone(&worker)), 65);
        let mut list = ListViewer::new(Rc::clone(&app_data), PortForwardsList::default(), view);
        list.table.update(worker.borrow_mut().get_port_forwards_list(&namespace));
        let header = ListHeader::new(Rc::clone(&app_data), list.table.len())
            .with_kind(VIEW_NAME)
            .with_namespace(namespace.as_str())
            .with_scope(Scope::Namespaced)
            .with_hide_previous(true);

        Self {
            header,
            list,
            app_data,
            namespace,
            worker,
            last_mouse_click: None,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            filter,
            footer_tx,
            is_closing: false,
        }
    }

    /// Sets filter on the port forwards list.
    pub fn set_filter(&mut self, value: String) {
        self.filter.set_value(value);
        self.update_filter();
    }

    /// Updates filter on the port forwards list.
    pub fn update_filter(&mut self) {
        let value = self.filter.value();
        self.header.show_filtered_icon(!value.is_empty());
        if value.is_empty() {
            if self.list.table.is_filtered() {
                self.list.table.set_filter(None);
                self.header.set_count(self.list.table.len());
            }
        } else if !self.list.table.is_filtered() || self.list.table.filter().is_some_and(|f| f != value) {
            self.list.table.set_filter(Some(value.to_owned()));
            self.header.set_count(self.list.table.len());
        }
    }

    /// Shows command palette.
    fn show_command_palette(&mut self) {
        let mut builder = ActionsListBuilder::from_kinds(self.app_data.borrow().kinds.as_deref())
            .with_back()
            .with_quit()
            .with_filter_action("filter")
            .with_pin_filter_action("pin_filter");

        if !self.list.table.is_empty() {
            builder.add_action(
                ActionItem::action("stop stale", "cleanup").with_description("stops all stale port forwarding rules"),
                Some(KeyCommand::PortForwardsCleanup),
            );
            if self.list.table.is_anything_selected() {
                builder.add_action(
                    ActionItem::action("stop", "stop_selected").with_description("stops selected port forwarding rules"),
                    Some(KeyCommand::NavigateDelete),
                );
            }
        }

        builder = builder.with_aliases(&self.app_data.borrow().config.aliases);
        let actions = builder.build(Some(&self.app_data.borrow().key_bindings));
        self.command_palette =
            CommandPalette::new(Rc::clone(&self.app_data), actions, 65).with_highlighted_position(self.last_mouse_click.take());
        self.command_palette.show();
        self.footer_tx.hide_hint();
    }

    /// Shows menu for right mouse button.
    fn show_mouse_menu(&mut self, x: u16, y: u16) {
        if !self.app_data.borrow().is_connected() {
            return;
        }

        let mut builder = ActionsListBuilder::default()
            .with_menu_action(ActionItem::back())
            .with_menu_action(ActionItem::command_palette());

        if !self.list.table.is_empty() {
            if self.list.table.is_anything_selected() {
                builder.add_menu_action(ActionItem::menu(1, " stop ␝selected␝", "stop_selected"));
            }

            let caption = if self.list.table.is_filtered() {
                " stop ␝stale []␝"
            } else {
                " stop ␝stale␝"
            };
            builder.add_menu_action(ActionItem::menu(2, caption, "cleanup"));
        }

        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(None), 22).to_mouse_menu();
        self.command_palette.show_at((x.saturating_sub(3), y).into());
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
            .show_info("Selected port forwarding rules have been stopped", 3_000);
    }

    /// Creates new stop dialog.
    fn new_stop_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "Are you sure you want to stop the selected port forwarding rules?".to_owned(),
            vec![
                Button::new("Stop", ResponseEvent::Action("delete"), &colors.modal.btn_delete),
                Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
            ],
            60,
            colors.modal.text,
        )
        .with_highlighted_position(self.last_mouse_click.take())
    }

    /// Shows dialog to stop stale port forwarding rules.
    fn ask_stop_stale_port_forwards(&mut self) {
        if !self.list.table.is_empty() {
            self.modal = self.new_cleanup_dialog();
            self.modal.show();
        }
    }

    /// Stops stale port forwarding rules.\
    /// **Note** that it stops only visible (full or filtered list) tasks.
    fn stop_stale_port_forwards(&mut self) {
        if !self.list.table.is_filtered() && self.namespace.is_all() {
            self.worker.borrow_mut().stop_stale_port_forwards(None);
            self.footer_tx
                .show_info("All stale port forwarding rules have been stopped", 3_000);
        } else {
            let filtered = Some(self.list.table.to_container_vec());
            self.worker.borrow_mut().stop_stale_port_forwards(filtered.as_deref());
            self.footer_tx
                .show_info("Stale port forwarding rules stopped for pods in current view", 3_000);
        }
    }

    /// Creates new cleanup stale pods dialog.
    fn new_cleanup_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;
        let kind = if !self.list.table.is_filtered() && self.namespace.is_all() {
            "all"
        } else {
            "visible"
        };

        Dialog::new(
            format!("Are you sure you want to stop {kind} port forwarding rules for pods that no longer exist? "),
            vec![
                Button::new("Stop", ResponseEvent::Action("cleanup"), &colors.modal.btn_delete),
                Button::new("Cancel", ResponseEvent::Cancelled, &colors.modal.btn_cancel),
            ],
            60,
            colors.modal.text,
        )
        .with_highlighted_position(self.last_mouse_click.take())
    }

    fn get_mouse_menu_position(&self, line_no: u16, resource_name: &str) -> Position {
        self.list
            .table
            .table
            .get_mouse_menu_position(line_no, resource_name, self.list.area)
    }

    fn process_command_palette_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        match self.command_palette.process_event(event) {
            ResponseEvent::ChangeKind(kind) => {
                self.is_closing = true;
                ResponseEvent::ChangeKind(kind)
            },
            ResponseEvent::Action("stop_selected") => {
                self.last_mouse_click = event.position();
                self.ask_stop_port_forwards();
                ResponseEvent::Handled
            },
            ResponseEvent::Action("cleanup") => {
                self.last_mouse_click = event.position();
                self.ask_stop_stale_port_forwards();
                ResponseEvent::Handled
            },
            ResponseEvent::Action("palette") => {
                self.last_mouse_click = event.position();
                self.process_event(&TuiEvent::Command(KeyCommand::CommandPaletteOpen))
            },
            ResponseEvent::Action("filter") => {
                self.last_mouse_click = event.position();
                self.process_event(&TuiEvent::Command(KeyCommand::FilterOpen))
            },
            ResponseEvent::Action("pin_filter") => self.process_event(&TuiEvent::Command(KeyCommand::FilterPin)),
            response_event => response_event,
        }
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
        let (namespace, view) = get_current_namespace(&self.app_data);
        if self.namespace == namespace {
            return;
        }

        self.namespace = namespace;
        self.list.view = view;
        self.list
            .table
            .update(self.worker.borrow_mut().get_port_forwards_list(&self.namespace));

        if self.app_data.borrow().is_pinned {
            self.update_filter();
        } else {
            self.set_filter(String::new());
        }

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
            if self.filter.is_valid() {
                self.update_filter();
                self.filter.update_pinned_filter();
            }

            return ResponseEvent::Handled;
        }

        if self.modal.is_visible {
            let response = self.modal.process_event(event);
            if response.is_action("delete") {
                self.stop_selected_port_forwards();
            } else if response.is_action("cleanup") {
                self.stop_stale_port_forwards();
            }

            return ResponseEvent::Handled;
        }

        if self.command_palette.is_visible {
            let result = self.process_command_palette_event(event);
            if result != ResponseEvent::NotHandled {
                return result;
            }
        }

        if self.app_data.has_binding(event, KeyCommand::CommandPaletteOpen) {
            self.show_command_palette();
            return ResponseEvent::Handled;
        }

        if let TuiEvent::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::RightClick
            && self.list.area.contains(Position::new(mouse.column, mouse.row))
        {
            let line_no = mouse.row.saturating_sub(self.list.area.y);
            if !self.list.table.highlight_item_by_line(line_no) {
                self.list.table.unhighlight_item();
            }
            self.show_mouse_menu(mouse.column, mouse.row);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::MouseMenuOpen)
            && let Some(line_no) = self.list.table.get_highlighted_item_line_no()
            && let Some(item_name) = self.list.table.get_highlighted_item_name()
        {
            let pos = self.get_mouse_menu_position(line_no, item_name);
            self.show_mouse_menu(pos.x, pos.y);
            return ResponseEvent::Handled;
        }

        if self.app_data.has_binding(event, KeyCommand::FilterPin) {
            return self.filter.toggle_pin();
        }

        if self.filter.is_reset_filter_event(event) {
            self.filter.reset();
            self.update_filter();
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

        if self.app_data.has_binding(event, KeyCommand::PortForwardsCleanup) {
            self.ask_stop_stale_port_forwards();
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
