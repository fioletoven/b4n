use crossterm::event::{KeyCode, KeyModifiers};
use delegate::delegate;
use kube::discovery::Scope;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::{collections::HashMap, rc::Rc};

use crate::{
    core::{ResourcesInfo, SharedAppData},
    kubernetes::{
        ALL_NAMESPACES, Kind, NAMESPACES, Namespace, ResourceRef,
        resources::{CONTAINERS, PODS, ResourceItem, ResourcesList, SECRETS},
        watchers::ObserverResult,
    },
    ui::{
        KeyCombination, Responsive, Table, ViewType,
        lists::Row,
        tui::ResponseEvent,
        views::{ListHeader, ListViewer},
    },
};

/// Resources table.
pub struct ResourcesTable {
    pub header: ListHeader,
    pub list: ListViewer<ResourcesList>,
    app_data: SharedAppData,
    highlight_next: Option<String>,
}

impl ResourcesTable {
    /// Creates a new resources table.
    pub fn new(app_data: SharedAppData) -> Self {
        let list = ListViewer::new(
            Rc::clone(&app_data),
            ResourcesList::default().with_filter_settings(Some("e")),
            ViewType::Compact,
        );
        let header = ListHeader::new(Rc::clone(&app_data), None, list.table.len());

        Self {
            header,
            list,
            app_data,
            highlight_next: None,
        }
    }

    /// Resets all table data.
    pub fn reset(&mut self) {
        self.list.table.clear();
        self.header.set_count(0);
        self.header.show_filtered_icon(false);
    }

    /// Sets initial kubernetes resources data for [`ResourcesTable`].
    pub fn set_resources_info(&mut self, context: String, namespace: Namespace, version: String, scope: Scope) {
        if scope == Scope::Cluster || !namespace.is_all() {
            self.set_view(ViewType::Compact);
        } else {
            self.set_view(ViewType::Full);
        }

        self.app_data.borrow_mut().current = ResourcesInfo::from(context, namespace, version, scope);
    }

    /// Remembers resource name that will be highlighted for next background observer result.
    pub fn highlight_next(&mut self, resource_to_select: Option<String>) {
        self.highlight_next = resource_to_select;
    }

    delegate! {
        to self.list.table {
            pub fn deselect_all(&mut self);
            pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&ResourceItem>;
            pub fn has_containers(&self) -> bool;
        }
    }

    /// Gets current kind (plural) for resources listed in [`ResourcesTable`].
    pub fn kind_plural(&self) -> &str {
        &self.list.table.data.kind_plural
    }

    /// Gets current scope for resources listed in [`ResourcesTable`].
    pub fn scope(&self) -> &Scope {
        &self.list.table.data.scope
    }

    /// Gets resources group.
    pub fn group(&self) -> &str {
        &self.list.table.data.group
    }

    /// Returns resources kind.
    pub fn get_kind(&self) -> Kind {
        Kind::new(&self.list.table.data.kind_plural, &self.list.table.data.group)
    }

    /// Returns [`ResourceRef`] for currently highlighted item.
    pub fn get_resource_ref(&self, prefer_container: bool) -> Option<ResourceRef> {
        self.list
            .table
            .get_highlighted_resource()
            .and_then(|r| self.resource_ref_from(r, prefer_container))
    }

    /// Sets namespace for [`ResourcesTable`].
    pub fn set_namespace(&mut self, namespace: Namespace) {
        self.set_view(if namespace.is_all() {
            ViewType::Full
        } else {
            ViewType::Compact
        });

        if namespace.is_all() || !self.app_data.borrow().current.is_namespace_equal(&namespace) {
            self.app_data.borrow_mut().current.set_namespace(namespace);
            self.list.table.deselect_all();
        }
    }

    /// Sets list view for [`ResourcesTable`].
    pub fn set_view(&mut self, view: ViewType) {
        self.list.view = if self.has_containers() { ViewType::Compact } else { view };
    }

    /// Sets filter on the resources list.
    pub fn set_filter(&mut self, value: &str) {
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

    /// Updates resources list with a new data from [`ObserverResult`].
    pub fn update_resources_list(&mut self, result: ObserverResult<ResourceItem>) {
        if matches!(result, ObserverResult::InitDone) {
            if let Some(name) = self.highlight_next.as_deref() {
                self.list.table.highlight_item_by_name(name);
                self.highlight_next = None;
            } else {
                self.list.table.highlight_first_item();
            }
        }

        if self.list.table.update(result) {
            let current = &mut self.app_data.borrow_mut().current;
            current.update_from(&self.list.table.data);
            self.header.set_count(self.list.table.len());
        } else {
            self.header.set_count(self.list.table.len());
        }
    }

    /// Process UI key event.
    pub fn process_key(&mut self, key: KeyCombination) -> ResponseEvent {
        self.highlight_next = None;

        if key.code == KeyCode::Esc {
            return self.process_esc_key();
        }

        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('f') {
            return ResponseEvent::ShowPortForwards;
        }

        let response = self.list.process_key(key);
        if response != ResponseEvent::NotHandled {
            response
        } else {
            self.process_highlighted_resource_key(key)
        }
    }

    fn process_highlighted_resource_key(&mut self, key: KeyCombination) -> ResponseEvent {
        if let Some(resource) = self.list.table.get_highlighted_resource() {
            if key.code == KeyCode::Enter {
                return self.process_enter_key(resource);
            }

            if key.code == KeyCode::Char('y') || (key.code == KeyCode::Char('x') && self.kind_plural() == SECRETS) {
                return self.process_view_yaml(resource, key.code == KeyCode::Char('x'));
            }

            let is_container_name_known = self.kind_plural() == CONTAINERS
                || (self.kind_plural() == PODS && resource.data.as_ref().is_some_and(|d| d.one_container.is_some()));
            if is_container_name_known {
                if key.code == KeyCode::Char('f') {
                    return self.process_view_ports(resource);
                }

                if key.code == KeyCode::Char('l') {
                    return self.process_view_logs(resource, false);
                }

                if key.code == KeyCode::Char('p') {
                    return self.process_view_logs(resource, true);
                }

                if key.code == KeyCode::Char('s') {
                    return self.process_open_shell(resource);
                }
            } else if self.kind_plural() == PODS
                && (key.code == KeyCode::Char('f') || key.code == KeyCode::Char('l') || key.code == KeyCode::Char('p'))
            {
                return self.process_enter_key(resource);
            }
        }

        ResponseEvent::NotHandled
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

    fn process_esc_key(&self) -> ResponseEvent {
        match self.kind_plural() {
            NAMESPACES => ResponseEvent::Handled,
            CONTAINERS => {
                let to_select = self.app_data.borrow().current.name.clone();
                ResponseEvent::ChangeKindAndSelect(PODS.to_owned(), to_select)
            },
            _ => ResponseEvent::ViewNamespaces,
        }
    }

    fn process_enter_key(&self, resource: &ResourceItem) -> ResponseEvent {
        match self.kind_plural() {
            NAMESPACES => ResponseEvent::Change(PODS.to_owned(), resource.name.clone()),
            PODS => ResponseEvent::ViewContainers(resource.name.clone(), resource.namespace.clone().unwrap_or_default()),
            CONTAINERS => self.process_view_logs(resource, false),
            _ => self.process_view_yaml(resource, false),
        }
    }

    fn process_view_ports(&self, resource: &ResourceItem) -> ResponseEvent {
        self.resource_ref_from(resource, true)
            .map_or(ResponseEvent::NotHandled, ResponseEvent::ListResourcePorts)
    }

    fn process_view_logs(&self, resource: &ResourceItem, previous: bool) -> ResponseEvent {
        let resource = self.resource_ref_from(resource, true);
        if previous {
            resource.map_or(ResponseEvent::NotHandled, ResponseEvent::ViewPreviousLogs)
        } else {
            resource.map_or(ResponseEvent::NotHandled, ResponseEvent::ViewLogs)
        }
    }

    fn process_open_shell(&self, resource: &ResourceItem) -> ResponseEvent {
        self.resource_ref_from(resource, true)
            .map_or(ResponseEvent::NotHandled, ResponseEvent::OpenShell)
    }

    fn process_view_yaml(&self, resource: &ResourceItem, decode: bool) -> ResponseEvent {
        self.resource_ref_from(resource, false)
            .map_or(ResponseEvent::NotHandled, |r| ResponseEvent::ViewYaml(r, decode))
    }

    fn resource_ref_from(&self, resource: &ResourceItem, prefer_container: bool) -> Option<ResourceRef> {
        if self.kind_plural() == CONTAINERS {
            if let Some(name) = self.app_data.borrow().current.name.clone() {
                return Some(ResourceRef::container(
                    name,
                    resource.namespace.clone().into(),
                    resource.name.clone(),
                ));
            }
        } else if self.kind_plural() == PODS && prefer_container {
            if let Some(container) = resource.data.as_ref().and_then(|d| d.one_container.as_deref()) {
                return Some(ResourceRef::container(
                    resource.name.clone(),
                    resource.namespace.clone().into(),
                    container.to_owned(),
                ));
            }
        } else if resource.name() != ALL_NAMESPACES && resource.group() != NAMESPACES {
            return Some(ResourceRef::named(
                self.get_kind(),
                resource.group().into(),
                resource.name().to_owned(),
            ));
        }

        None
    }
}
