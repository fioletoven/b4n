use b4n_config::keys::KeyCommand;
use b4n_kube::{
    ALL_NAMESPACES, CONTAINERS, DAEMON_SETS, DEPLOYMENTS, EVENTS, JOBS, Kind, NAMESPACES, NODES, Namespace, ObserverResult, PODS,
    REPLICA_SETS, ResourceRef, ResourceRefFilter, SECRETS, SERVICES, STATEFUL_SETS,
};
use b4n_list::Row;
use b4n_tui::{MouseEventKind, ResponseEvent, Responsive, ScopeData, Table, TuiEvent, grid::ViewType};
use crossterm::event::KeyModifiers;
use delegate::delegate;
use kube::discovery::Scope;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::{collections::HashMap, rc::Rc};

use crate::core::{PreviousData, ResourcesInfo, SharedAppData, SharedAppDataExt};
use crate::kubernetes::resources::{ResourceItem, ResourcesList};
use crate::ui::viewers::{ListHeader, ListViewer};

/// Actions to perform on the next table refresh.
#[derive(Default)]
pub struct NextRefreshActions {
    pub highlight_item: Option<String>,
    pub apply_filter: Option<String>,
    pub sort_info: Option<(usize, bool)>,
    pub offset: Option<usize>,
    pub clear_header_scope: bool,
}

impl NextRefreshActions {
    /// Creates new [`NextRefreshActions`] instance that will highlight `resource_name` on next refresh.
    pub fn highlight(resource_name: Option<String>) -> Self {
        NextRefreshActions {
            highlight_item: resource_name,
            ..Default::default()
        }
    }

    /// Creates new [`NextRefreshActions`] instance from the [`PreviousData`] object.
    pub fn from_previous(previous: &PreviousData) -> Self {
        NextRefreshActions {
            highlight_item: previous.highlighted().map(String::from),
            apply_filter: previous.filter.as_deref().map(String::from),
            sort_info: Some(previous.sort_info),
            offset: Some(previous.offset),
            clear_header_scope: false,
        }
    }

    /// Clears the [`NextRefreshActions`] object.
    pub fn clear(&mut self) {
        self.highlight_item = None;
        self.apply_filter = None;
    }
}

/// Resources table.
pub struct ResourcesTable {
    pub header: ListHeader,
    pub list: ListViewer<ResourcesList>,
    app_data: SharedAppData,
    next_refresh: NextRefreshActions,
}

impl ResourcesTable {
    /// Creates a new resources table.
    pub fn new(app_data: SharedAppData) -> Self {
        let list = ListViewer::new(
            Rc::clone(&app_data),
            ResourcesList::default().with_filter_settings(Some("e")),
            ViewType::Compact,
        );
        let header = ListHeader::new(Rc::clone(&app_data), list.table.len());

        Self {
            header,
            list,
            app_data,
            next_refresh: NextRefreshActions::default(),
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

    /// Returns [`NextRefreshActions`] object.
    pub fn next_refresh(&self) -> &NextRefreshActions {
        &self.next_refresh
    }

    /// Remembers actions that will be applied for next background observer result.
    pub fn set_next_refresh(&mut self, actions: NextRefreshActions) {
        self.next_refresh = actions;
    }

    /// Remembers resource name that will be highlighted for next background observer result.
    pub fn set_next_highlight(&mut self, resource_to_select: Option<String>) {
        self.next_refresh.highlight_item = resource_to_select;
    }

    /// Remembers if header scope should be reset to default for next background observer result.
    pub fn clear_header_scope(&mut self, clear_on_next: bool) {
        self.next_refresh.clear_header_scope = clear_on_next;
    }

    delegate! {
        to self.list.table {
            pub fn deselect_all(&mut self);
            pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&ResourceItem>;
            pub fn has_containers(&self) -> bool;
            pub fn is_filtered(&self) -> bool;
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

    /// Gets resources version.
    pub fn version(&self) -> &str {
        &self.list.table.data.version
    }

    /// Returns resources kind.
    pub fn get_kind(&self) -> Kind {
        Kind::new(
            &self.list.table.data.kind_plural,
            &self.list.table.data.group,
            &self.list.table.data.version,
        )
    }

    /// Returns resources kind.\
    /// **Note** that it returns `pods` if the currently shown items are containers.
    pub fn get_kind_for_selector(&self) -> Kind {
        if self.list.table.data.resource.is_container() {
            PODS.into()
        } else {
            self.get_kind()
        }
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
        let is_full = namespace.is_all() && self.app_data.borrow().current.scope == Scope::Namespaced;
        self.set_view(if is_full { ViewType::Full } else { ViewType::Compact });

        if namespace.is_all() || !self.app_data.borrow().current.is_namespace_equal(&namespace) {
            self.app_data.borrow_mut().current.set_namespace(namespace);
            self.list.table.deselect_all();
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
        let is_init = matches!(result, ObserverResult::Init(_));
        let is_init_done = matches!(result, ObserverResult::InitDone);

        if self.list.table.update(result) {
            let current = &mut self.app_data.borrow_mut().current;
            current.update_from(&self.list.table.data);
        }

        if is_init {
            if let Some(filter) = self.next_refresh.apply_filter.take() {
                self.set_filter(&filter);
            } else {
                self.set_filter("");
            }

            if let Some((column_no, is_descending)) = self.next_refresh.sort_info.take() {
                self.list.table.table.header.set_sort_info(column_no, is_descending);
            }

            if self.next_refresh.clear_header_scope {
                self.header.set_scope(None);
                self.next_refresh.clear_header_scope = false;
            }

            if let Some(offset) = self.next_refresh.offset.take() {
                let current_width = usize::from(self.list.area.width);
                // we need to refresh header here, as init data invalidates its cache.
                self.list.table.refresh_header(self.list.view, current_width);
                self.list.table.table.set_offset(offset);
            }
        }

        if is_init_done {
            if let Some(name) = self.next_refresh.highlight_item.take() {
                self.list.table.highlight_item_by_name(&name);
            } else {
                self.list.table.highlight_first_item();
            }
        }

        self.header.set_count(self.list.table.len());
    }

    /// Process UI key/mouse event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.next_refresh.clear();

        if self.app_data.has_binding(event, KeyCommand::NavigateBack) {
            return self.process_esc_key();
        }

        if self.app_data.has_binding(event, KeyCommand::PortForwardsOpen) {
            return ResponseEvent::ShowPortForwards;
        }

        let response = self.list.process_event(event);
        if response == ResponseEvent::NotHandled {
            return self.process_highlighted_resource_event(event);
        }

        response
    }

    fn process_highlighted_resource_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let Some(resource) = self.list.table.get_highlighted_resource() {
            if self.app_data.has_binding(event, KeyCommand::NavigateInto) {
                return self.process_enter_key(resource);
            }

            if let Some(line_no) = event.get_clicked_line_no(MouseEventKind::LeftDoubleClick, KeyModifiers::NONE, self.list.area)
                && usize::from(line_no) < self.list.table.len()
            {
                return self.process_enter_key(resource);
            }

            if self.app_data.has_binding(event, KeyCommand::YamlOpen)
                || (self.app_data.has_binding(event, KeyCommand::YamlDecode) && self.kind_plural() == SECRETS)
            {
                return self.process_view_yaml(resource, self.app_data.has_binding(event, KeyCommand::YamlDecode));
            }

            let is_container = self.kind_plural() == CONTAINERS;
            if self.app_data.has_binding(event, KeyCommand::EventsShow) {
                if !is_container && resource.name() != ALL_NAMESPACES {
                    return self.process_view_events(resource);
                }

                return ResponseEvent::NotHandled;
            }

            if self.app_data.has_binding(event, KeyCommand::InvolvedObjectShow) {
                if let Some(involved) = &resource.involved_object {
                    return ResponseEvent::ViewInvolved(
                        involved.kind.clone().into(),
                        involved.namespace.clone().into(),
                        Some(involved.name.clone()),
                    );
                }

                return ResponseEvent::NotHandled;
            }

            let is_container_name_known =
                is_container || (self.kind_plural() == PODS && resource.data.as_ref().is_some_and(|d| d.tags.len() == 1));
            if is_container_name_known {
                if self.app_data.has_binding(event, KeyCommand::PortForwardsCreate) {
                    return self.process_view_ports(resource);
                }

                if self.app_data.has_binding(event, KeyCommand::LogsOpen) {
                    return self.process_view_logs(resource, false);
                }

                if self.app_data.has_binding(event, KeyCommand::PreviousLogsOpen) {
                    return self.process_view_logs(resource, true);
                }

                if self.app_data.has_binding(event, KeyCommand::ShellOpen) {
                    return self.process_open_shell(resource);
                }
            } else if self.kind_plural() == PODS
                && (self.app_data.has_binding(event, KeyCommand::PortForwardsCreate)
                    || self.app_data.has_binding(event, KeyCommand::LogsOpen)
                    || self.app_data.has_binding(event, KeyCommand::PreviousLogsOpen)
                    || self.app_data.has_binding(event, KeyCommand::ShellOpen))
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
        if self.kind_plural() == NAMESPACES {
            ResponseEvent::Handled
        } else if !self.app_data.borrow().previous.is_empty() {
            ResponseEvent::ViewPreviousResource
        } else {
            ResponseEvent::ViewNamespaces
        }
    }

    fn process_enter_key(&self, resource: &ResourceItem) -> ResponseEvent {
        match self.kind_plural() {
            NODES => ResourcesTable::process_view_nodes(resource),
            JOBS => self.process_view_jobs(resource),
            DEPLOYMENTS => self.process_view_selector(resource, REPLICA_SETS),
            SERVICES | REPLICA_SETS | STATEFUL_SETS | DAEMON_SETS => self.process_view_selector(resource, PODS),
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
            if let Some(name) = self.app_data.borrow().current.resource.name.clone() {
                return Some(ResourceRef::container(
                    name,
                    resource.namespace.clone().into(),
                    resource.name.clone(),
                ));
            }
        } else if self.kind_plural() == PODS && prefer_container {
            if let Some(data) = resource.data.as_ref()
                && data.tags.len() == 1
            {
                return Some(ResourceRef::container(
                    resource.name.clone(),
                    resource.namespace.clone().into(),
                    data.tags[0].clone(),
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

    fn process_view_events(&self, resource: &ResourceItem) -> ResponseEvent {
        let scope = ScopeData {
            header: self.app_data.borrow().current.scope.clone(),
            list: Scope::Cluster,
            filter: ResourceRefFilter::involved(resource.name.clone(), &resource.uid),
        };
        ResponseEvent::ViewScoped(EVENTS.to_owned(), resource.namespace.clone(), None, scope)
    }

    fn process_view_nodes(resource: &ResourceItem) -> ResponseEvent {
        let filter = ResourceRefFilter::node(resource.name.clone(), &resource.name);
        ResponseEvent::ViewScoped(PODS.to_owned(), None, None, ScopeData::namespace_visible(filter))
    }

    fn process_view_jobs(&self, resource: &ResourceItem) -> ResponseEvent {
        let scope = ScopeData {
            header: self.app_data.borrow().current.scope.clone(),
            list: Scope::Cluster,
            filter: ResourceRefFilter::job(resource.name.clone(), &resource.name),
        };
        ResponseEvent::ViewScoped(PODS.to_owned(), resource.namespace.clone(), None, scope)
    }

    fn process_view_selector(&self, resource: &ResourceItem, target: &str) -> ResponseEvent {
        if let Some(data) = &resource.data
            && !data.tags.is_empty()
            && !data.tags[0].is_empty()
        {
            let filter = ResourceRefFilter::labels(resource.name.clone(), data.tags[0].clone());
            ResponseEvent::ViewScoped(
                target.to_owned(),
                resource.namespace.clone(),
                None,
                ScopeData::namespace_hidden(filter),
            )
        } else {
            self.process_view_yaml(resource, false)
        }
    }
}
