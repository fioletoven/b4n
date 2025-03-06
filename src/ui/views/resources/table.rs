use crossterm::event::{KeyCode, KeyEvent};
use delegate::delegate;
use kube::discovery::Scope;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::{collections::HashMap, rc::Rc};

use crate::{
    app::{
        ObserverResult, ResourcesInfo, SharedAppData,
        lists::{ResourcesList, Row},
    },
    kubernetes::{ALL_NAMESPACES, NAMESPACES, Namespace, resources::Resource},
    ui::{Responsive, Table, ViewType, tui::ResponseEvent},
};

use super::{HeaderPane, ListPane};

/// Resources table.
pub struct ResourcesTable {
    pub header: HeaderPane,
    pub list: ListPane<ResourcesList>,
    app_data: SharedAppData,
    highlight_next: Option<String>,
}

impl ResourcesTable {
    /// Creates a new resources table.
    pub fn new(app_data: SharedAppData) -> Self {
        let header = HeaderPane::new(Rc::clone(&app_data));
        let list = ListPane::new(
            Rc::clone(&app_data),
            ResourcesList::default().with_filter_settings(Some("")),
            ViewType::Compact,
        );

        Self {
            header,
            list,
            app_data,
            highlight_next: None,
        }
    }

    /// Resets all table data.
    pub fn reset(&mut self) {
        self.list.items.clear();
        self.header.show_filtered_icon(false);
    }

    /// Sets initial kubernetes resources data for [`ResourcesTable`].
    pub fn set_resources_info(&mut self, context: String, namespace: Namespace, version: String, scope: Scope) {
        self.list.view = ViewType::Full;
        if scope == Scope::Cluster || !namespace.is_all() {
            self.list.view = ViewType::Compact;
        }

        self.app_data.borrow_mut().current = ResourcesInfo::from(context, namespace, version, scope);
    }

    /// Remembers resource name that will be highlighted for next background observer result.
    pub fn highlight_next(&mut self, resource_to_select: Option<String>) {
        self.highlight_next = resource_to_select;
    }

    delegate! {
        to self.list.items {
            pub fn deselect_all(&mut self);
            pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&Resource>;
        }
    }

    /// Gets current kind (plural) for resources listed in [`ResourcesTable`].
    pub fn kind_plural(&self) -> &str {
        &self.list.items.kind_plural
    }

    /// Gets current scope for resources listed in [`ResourcesTable`].
    pub fn scope(&self) -> &Scope {
        &self.list.items.scope
    }

    /// Gets resources group.
    pub fn group(&self) -> &str {
        &self.list.items.group
    }

    /// Sets namespace for [`ResourcesTable`].
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

    /// Sets list view for [`ResourcesTable`].
    pub fn set_view(&mut self, view: ViewType) {
        self.list.view = view;
    }

    /// Sets filter on the resources list.
    pub fn set_filter(&mut self, value: &str) {
        self.header.show_filtered_icon(!value.is_empty());
        if value.is_empty() {
            if self.list.items.is_filtered() {
                self.list.items.filter(None);
                self.app_data.borrow_mut().current.count = self.list.items.len();
            }
        } else if !self.list.items.is_filtered() || self.list.items.get_filter().is_some_and(|f| f != value) {
            self.list.items.filter(Some(value.to_owned()));
            self.app_data.borrow_mut().current.count = self.list.items.len();
        }
    }

    /// Updates resources list with a new data from [`ObserverResult`].
    pub fn update_resources_list(&mut self, result: Box<ObserverResult>) {
        if self.list.items.update(result) {
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

    /// Process UI key event.
    pub fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if key.code == KeyCode::Enter && self.kind_plural() == NAMESPACES {
            if let Some(selected_namespace) = self.list.items.get_highlighted_item_name() {
                return ResponseEvent::Change("pods".to_owned(), selected_namespace.to_owned());
            }
        }

        if key.code == KeyCode::Esc && self.kind_plural() != NAMESPACES {
            return ResponseEvent::ViewNamespaces;
        }

        if key.code == KeyCode::Char('y') {
            if let Some(selected_resource) = self.list.items.get_highlighted_resource() {
                if selected_resource.name() != ALL_NAMESPACES && selected_resource.group() != NAMESPACES {
                    return ResponseEvent::ViewYaml(selected_resource.name().to_owned(), selected_resource.group().to_owned());
                }
            }
        }

        self.list.process_key(key)
    }

    /// Draws [`ResourcesTable`] on the provided frame and area.
    pub fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        self.header.draw(frame, layout[0]);
        self.list.draw(frame, layout[1]);
    }
}
