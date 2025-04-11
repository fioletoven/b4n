use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use delegate::delegate;
use kube::{config::NamedContext, discovery::Scope};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::{collections::HashMap, rc::Rc};

use crate::{
    app::{ObserverResult, SharedAppData, SharedBgWorker},
    kubernetes::{
        Namespace,
        kinds::{KindItem, KindsList},
        resources::{CONTAINERS, ResourceItem, ResourcesList},
    },
    ui::{
        Responsive, Table, ViewType,
        tui::{ResponseEvent, TuiEvent},
        widgets::{ActionItem, ActionsListBuilder, Button, CommandPalette, Dialog, Filter, Position, SideSelect},
    },
};

use super::ResourcesTable;

/// Resources view (main view) for `b4n`.
pub struct ResourcesView {
    app_data: SharedAppData,
    table: ResourcesTable,
    modal: Dialog,
    command_palette: CommandPalette,
    ns_selector: SideSelect<ResourcesList>,
    res_selector: SideSelect<KindsList>,
    filter: Filter,
}

impl ResourcesView {
    /// Creates a new resources view.
    pub fn new(app_data: SharedAppData, worker: SharedBgWorker) -> Self {
        let table = ResourcesTable::new(Rc::clone(&app_data));
        let ns_selector = SideSelect::new(
            "NAMESPACE",
            Rc::clone(&app_data),
            ResourcesList::default(),
            Position::Left,
            ResponseEvent::ChangeNamespace,
            30,
        );
        let res_selector = SideSelect::new(
            "RESOURCE",
            Rc::clone(&app_data),
            KindsList::default(),
            Position::Right,
            ResponseEvent::ChangeKind,
            35,
        );
        let filter = Filter::new(Rc::clone(&app_data), Some(worker), 60);

        Self {
            app_data,
            table,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            ns_selector,
            res_selector,
            filter,
        }
    }

    delegate! {
        to self.table {
            pub fn set_resources_info(&mut self, context: String, namespace: Namespace, version: String, scope: Scope);
            pub fn highlight_next(&mut self, resource_to_select: Option<String>);
            pub fn deselect_all(&mut self);
            pub fn kind_plural(&self) -> &str;
            pub fn scope(&self) -> &Scope;
            pub fn group(&self) -> &str;
            pub fn get_kind_with_group(&self) -> String;
            pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&ResourceItem>;
            pub fn set_namespace(&mut self, namespace: Namespace);
            pub fn set_view(&mut self, view: ViewType);
            pub fn update_resources_list(&mut self, result: ObserverResult);
        }
    }

    /// Resets all data for a resources view.
    pub fn reset(&mut self) {
        self.table.reset();
        self.filter.reset();
        self.ns_selector.select.items.clear();
        self.ns_selector.hide();
        self.res_selector.select.items.clear();
        self.res_selector.hide();
    }

    /// Clears data in the list.
    pub fn clear_list_data(&mut self) {
        self.table.reset();
        self.filter.reset();
    }

    /// Updates namespaces list with a new data from [`ObserverResult`].
    pub fn update_namespaces_list(&mut self, result: ObserverResult) {
        self.ns_selector.select.items.update(result);
    }

    /// Updates kinds list with a new data.
    pub fn update_kinds_list(&mut self, kinds: Option<Vec<KindItem>>) {
        self.res_selector.select.items.update(kinds, 1, false);
    }

    /// Shows delete resources dialog if anything is selected.
    pub fn ask_delete_resources(&mut self) {
        if self.table.list.items.is_anything_selected() && !self.table.has_containers() {
            self.modal = self.new_delete_dialog();
            self.modal.show();
        }
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: Vec<NamedContext>) {
        let actions_list = ActionsListBuilder::from_contexts(&list).build();
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions_list, 60);
        self.command_palette.set_prompt("context");
        self.command_palette.select(&self.app_data.borrow().current.context);
        self.command_palette.show();
    }

    /// Process TUI event.
    pub fn process_event(&mut self, event: TuiEvent) -> ResponseEvent {
        let TuiEvent::Key(key) = event;

        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return ResponseEvent::ExitApplication;
        }

        if self.modal.is_visible {
            return self.modal.process_key(key);
        }

        if self.command_palette.is_visible {
            return self
                .command_palette
                .process_key(key)
                .if_action_then("show_yaml", || self.table.process_key(KeyEvent::from(KeyCode::Char('y'))))
                .if_action_then("decode_yaml", || self.table.process_key(KeyEvent::from(KeyCode::Char('x'))))
                .if_action_then("show_logs", || self.table.process_key(KeyEvent::from(KeyCode::Char('l'))))
                .if_action_then("show_plogs", || self.table.process_key(KeyEvent::from(KeyCode::Char('p'))));
        }

        if !self.app_data.borrow().is_connected {
            self.process_command_palette_events(key);
            return ResponseEvent::Handled;
        }

        if self.ns_selector.is_visible {
            return self.ns_selector.process_key(key);
        }

        if self.res_selector.is_visible {
            return self.res_selector.process_key(key);
        }

        if self.filter.is_visible {
            return self.filter.process_key(key);
        }

        if key.code == KeyCode::Left && self.table.scope() == &Scope::Namespaced && !self.table.has_containers() {
            self.ns_selector
                .show_selected(self.app_data.borrow().current.namespace.as_str(), "");
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Right {
            self.res_selector.show_selected(self.table.kind_plural(), self.table.group());
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
            self.ask_delete_resources();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Esc && !self.filter.value().is_empty() {
            self.filter.reset();
            return ResponseEvent::Handled;
        }

        if key.code == KeyCode::Char('/') {
            self.filter.show();
            return ResponseEvent::Handled;
        }

        self.process_command_palette_events(key);

        self.table.process_key(key)
    }

    /// Processes disconnection state.
    pub fn process_disconnection(&mut self) {
        self.ns_selector.hide();
        self.res_selector.hide();
        self.command_palette.hide();
    }

    /// Draws [`ResourcesView`] on the provided frame and area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.table.set_filter(self.filter.value());
        self.table.draw(frame, area);

        self.modal.draw(frame, frame.area());
        self.command_palette.draw(frame, frame.area());
        self.filter.draw(frame, frame.area());

        self.draw_selectors(frame, area);
    }

    /// Draws namespace / resource selector located on the left / right of the resources list.
    fn draw_selectors(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if self.ns_selector.is_visible || self.res_selector.is_visible {
            let bottom = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
                .split(area);

            self.ns_selector.draw(frame, bottom[1]);
            self.res_selector.draw(frame, bottom[1]);
        }
    }

    fn process_command_palette_events(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char(':') || key.code == KeyCode::Char('>') {
            let is_containers = self.table.kind_plural() == CONTAINERS;
            let mut builder = if self.app_data.borrow().is_connected {
                ActionsListBuilder::from_kinds(&self.res_selector.select.items.list).with_resources_actions(!is_containers)
            } else {
                ActionsListBuilder::default().with_resources_actions(false)
            };

            if is_containers {
                builder = builder
                    .with_action(
                        ActionItem::new("show logs")
                            .with_description("shows container logs")
                            .with_aliases(&["logs"])
                            .with_response(ResponseEvent::Action("show_logs")),
                    )
                    .with_action(
                        ActionItem::new("show previous logs")
                            .with_description("shows container previous logs")
                            .with_aliases(&["previous"])
                            .with_response(ResponseEvent::Action("show_plogs")),
                    );
            } else {
                builder = builder.with_action(
                    ActionItem::new("show YAML")
                        .with_description("shows YAML of the selected resource")
                        .with_aliases(&["yaml"])
                        .with_response(ResponseEvent::Action("show_yaml")),
                );
            }

            if self.table.kind_plural() == "secrets" {
                builder = builder.with_action(
                    ActionItem::new("decode")
                        .with_description("shows decoded YAML of the selected secret")
                        .with_aliases(&["decode", "x"])
                        .with_response(ResponseEvent::Action("decode_yaml")),
                );
            }

            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), builder.build(), 60);
            self.command_palette.show();
        }
    }

    /// Creates new delete dialog.
    fn new_delete_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().theme.colors;

        Dialog::new(
            "Are you sure you want to delete the selected resources?".to_owned(),
            vec![
                Button::new(
                    "Delete".to_owned(),
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
