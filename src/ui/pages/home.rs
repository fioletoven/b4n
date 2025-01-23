use crossterm::event::{KeyCode, KeyModifiers};
use kube::{config::NamedContext, discovery::Scope};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use std::{collections::HashMap, rc::Rc};

use crate::{
    app::{
        lists::{ActionsList, KindsList, ResourcesList},
        ObserverResult, ResourcesInfo, SharedAppData,
    },
    kubernetes::{resources::Kind, Namespace, NAMESPACES},
    ui::{
        panes::{FooterPane, HeaderPane, ListPane},
        tui::{ResponseEvent, TuiEvent},
        widgets::{Button, CommandPalette, Dialog, Position, SideSelect},
        Responsive, Table, ViewType,
    },
};

/// Home page (main page) for `b4n`.
pub struct HomePage {
    app_data: SharedAppData,
    header: HeaderPane,
    list: ListPane<ResourcesList>,
    footer: FooterPane,
    modal: Dialog,
    command_palette: CommandPalette,
    ns_selector: SideSelect<ResourcesList>,
    res_selector: SideSelect<KindsList>,
    highlight_next: Option<String>,
}

impl HomePage {
    /// Creates a new home page.
    pub fn new(app_data: SharedAppData) -> Self {
        let header = HeaderPane::new(Rc::clone(&app_data));
        let list = ListPane::new(Rc::clone(&app_data), ResourcesList::default(), ViewType::Compact);
        let footer = FooterPane::new(Rc::clone(&app_data));

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

        Self {
            app_data,
            header,
            list,
            footer,
            modal: Dialog::default(),
            command_palette: CommandPalette::default(),
            ns_selector,
            res_selector,
            highlight_next: None,
        }
    }

    /// Resets all data on a home page.
    pub fn reset(&mut self) {
        self.list.items.clear();
        self.ns_selector.select.items.clear();
        self.res_selector.select.items.clear();
    }

    /// Sets initial kubernetes resources data for [`HomePage`].
    pub fn set_resources_info(&mut self, context: String, namespace: Namespace, version: String, scope: Scope) {
        self.list.view = ViewType::Full;
        if scope == Scope::Cluster || namespace.is_all() {
            self.list.view = ViewType::Compact;
        }

        self.app_data.borrow_mut().current = ResourcesInfo::from(context, namespace, version, scope);
    }

    /// Remembers resource name that will be highlighted for next background observer result.
    pub fn highlight_next(&mut self, resource_to_select: Option<String>) {
        self.highlight_next = resource_to_select;
    }

    /// Deselects all selected items for [`HomePage`].
    pub fn deselect_all(&mut self) {
        self.list.items.deselect_all();
    }

    /// Gets current kind (plural) for resources listed in [`HomePage`].
    pub fn kind_plural(&self) -> &str {
        &self.list.items.kind_plural
    }

    /// Gets current scope for resources listed in [`HomePage`].
    pub fn scope(&self) -> &Scope {
        &self.list.items.scope
    }

    /// Gets currently selected item names (grouped in [`HashMap`]) on [`HomePage`].
    pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>> {
        self.list.items.get_selected_items()
    }

    /// Sets namespace for [`HomePage`].
    pub fn set_namespace(&mut self, namespace: Namespace) {
        self.list.view = if namespace.is_all() {
            ViewType::Full
        } else {
            ViewType::Compact
        };

        if self.app_data.borrow().current.namespace != namespace {
            self.app_data.borrow_mut().current.namespace = namespace;
            self.list.items.deselect_all();
        }
    }

    /// Sets list view for [`HomePage`].
    pub fn set_view(&mut self, view: ViewType) {
        self.list.view = view;
    }

    /// Updates resources list with a new data from [`ObserverResult`].
    pub fn update_resources_list(&mut self, result: Option<ObserverResult>) {
        if result.is_none() {
            return;
        }

        if self.list.items.update(result, 1, false) {
            let mut data = self.app_data.borrow_mut();
            data.current.kind = self.list.items.kind.clone();
            data.current.kind_plural = self.list.items.kind_plural.clone();
            data.current.group = self.list.items.group.clone();
            data.current.scope = self.list.items.scope.clone();
            data.current.count = self.list.items.list.len();
        } else {
            self.app_data.borrow_mut().current.count = self.list.items.list.len();
        }

        if let Some(name) = self.highlight_next.as_deref() {
            self.list.items.highlight_item_by_name(name);
            self.highlight_next = None;
        }
    }

    /// Updates namespaces list with a new data from [`ObserverResult`].
    pub fn update_namespaces_list(&mut self, result: Option<ObserverResult>) {
        self.ns_selector.select.items.update(result, 1, false);
    }

    /// Updates kinds list with a new data.
    pub fn update_kinds_list(&mut self, kinds: Option<Vec<Kind>>) {
        self.res_selector.select.items.update(kinds, 1, false);
    }

    /// Shows delete resources dialog if anything is selected.
    pub fn ask_delete_resources(&mut self) {
        if self.list.items.is_anything_selected() {
            self.modal = self.new_delete_dialog();
            self.modal.show();
        }
    }

    /// Displays a list of available contexts to choose from.
    pub fn show_contexts_list(&mut self, list: Vec<NamedContext>) {
        self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), ActionsList::from_contexts(&list), 60);
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
            return self.command_palette.process_key(key);
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

        if key.code == KeyCode::Left && self.list.items.scope == Scope::Namespaced {
            self.ns_selector
                .show_selected(&self.app_data.borrow().current.namespace.as_str(), "");
        }

        if key.code == KeyCode::Right {
            self.res_selector
                .show_selected(&self.list.items.kind_plural, &self.list.items.group);
        }

        if key.code == KeyCode::Esc && self.list.items.kind_plural != NAMESPACES {
            return ResponseEvent::ViewNamespaces(self.app_data.borrow().current.namespace.as_str().into());
        }

        if key.code == KeyCode::Enter && self.list.items.kind_plural == NAMESPACES {
            if let Some(selected_namespace) = self.list.items.get_highlighted_item_name() {
                return ResponseEvent::Change("pods".to_owned(), selected_namespace.to_owned());
            }
        }

        if key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL {
            self.ask_delete_resources();
        }

        self.process_command_palette_events(key);

        self.list.process_key(key);

        ResponseEvent::Handled
    }

    fn process_command_palette_events(&mut self, key: crossterm::event::KeyEvent) {
        if key.code == KeyCode::Char(':') || key.code == KeyCode::Char('>') {
            let actions = if self.app_data.borrow().is_connected {
                ActionsList::from_kinds(&self.res_selector.select.items.list)
            } else {
                ActionsList::predefined(true)
            };
            self.command_palette = CommandPalette::new(Rc::clone(&self.app_data), actions, 60);
            self.command_palette.show();
        }
    }

    /// Draws [`HomePage`] on the provided frame.
    pub fn draw(&mut self, frame: &mut Frame<'_>) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)])
            .split(frame.area());

        self.header.draw(frame, layout[0]);
        self.list.draw(frame, layout[1]);
        self.footer.draw(frame, layout[2]);
        self.modal.draw(frame, frame.area());
        self.command_palette.draw(frame, frame.area());

        self.draw_selectors(frame, layout[0].union(layout[1]));
    }

    /// Draws namespace / resource selector located on the left / right of the resources list
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

    /// Creates new delete dialog.
    fn new_delete_dialog(&mut self) -> Dialog {
        let colors = &self.app_data.borrow().config.theme.colors;

        Dialog::new(
            "Are you sure you want to delete the selected resources?".to_owned(),
            vec![
                Button::new("Delete".to_owned(), ResponseEvent::DeleteResources, colors.modal.btn_delete),
                Button::new("Cancel".to_owned(), ResponseEvent::Cancelled, colors.modal.btn_cancel),
            ],
            60,
            colors.modal.colors,
        )
    }
}
