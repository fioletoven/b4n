use b4n_lists::FilterableList;
use delegate::delegate;
use std::{collections::HashMap, rc::Rc};

use crate::{
    kubernetes::{
        ALL_NAMESPACES, NAMESPACES, Namespace,
        resources::{CONTAINERS, ResourceFilterContext, ResourceItem},
        watchers::{InitData, ObserverResult},
    },
    ui::{
        ResponseEvent, Responsive, Table, TuiEvent, ViewType,
        colors::TextColors,
        lists::{Item, Row, TabularList},
        theme::Theme,
    },
};

/// Kubernetes resources list.
#[derive(Default)]
pub struct ResourcesList {
    pub data: InitData,
    pub table: TabularList<ResourceItem, ResourceFilterContext>,
}

impl ResourcesList {
    /// Sets filter settings for [`ResourcesList`].
    pub fn with_filter_settings(mut self, settings: Option<impl Into<String>>) -> Self {
        self.table.list.set_filter_settings(settings);
        self
    }

    /// Updates [`ResourcesList`] with new data from [`ObserverResult`] and sorts the new list if needed.\
    /// Returns `true` if the kind was changed during the update.
    pub fn update(&mut self, result: ObserverResult<ResourceItem>) -> bool {
        let (sort_by, is_descending) = self.table.header.sort_info();
        match result {
            ObserverResult::Init(init) => {
                self.update_kind(*init);
                let (sort_by, is_descending) = self.table.header.sort_info();
                self.sort(sort_by, is_descending);
                true
            },
            ObserverResult::InitDone => false,
            ObserverResult::Apply(resource) => {
                self.update_list(resource, false);
                self.sort(sort_by, is_descending);
                false
            },
            ObserverResult::Delete(resource) => {
                self.update_list(resource, true);
                self.sort(sort_by, is_descending);
                false
            },
        }
    }

    /// Returns `true` if the resources in the list are of a special type `containers`.
    pub fn has_containers(&self) -> bool {
        self.data.kind_plural == CONTAINERS
    }

    /// Returns `true` if the resources in the list are scoped.
    pub fn is_scoped(&self) -> bool {
        self.data.resource.filter.is_some()
    }

    /// Gets highlighted resource.
    pub fn get_highlighted_resource(&self) -> Option<&ResourceItem> {
        self.table.list.get_highlighted_item().map(|i| &i.data)
    }

    /// Gets specific resource.
    pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&ResourceItem> {
        self.table.list.items.as_ref().and_then(|items| {
            items
                .full_iter()
                .find(|i| i.data.name == name && i.data.namespace.as_deref() == namespace.as_option())
                .map(|i| &i.data)
        })
    }

    fn update_kind(&mut self, init: InitData) {
        self.data = init;
        self.table.header = ResourceItem::header(
            &self.data.kind,
            &self.data.group,
            self.data.crd.as_ref(),
            self.data.has_metrics,
            self.data.resource.is_filtered(),
        );
        self.table.list.clear();
        if self.data.kind_plural == NAMESPACES {
            self.table.list.items = Some(FilterableList::from(vec![Item::fixed(ResourceItem::new(ALL_NAMESPACES))]));
        }
    }

    /// Adds, updates or deletes `new_item` from the resources list.
    fn update_list(&mut self, new_item: ResourceItem, is_delete: bool) {
        if let Some(items) = &mut self.table.list.items {
            if is_delete {
                if let Some(index) = items.full_iter().position(|i| i.data.uid() == new_item.uid()) {
                    items.full_remove(index);
                }
            } else if let Some(old_item) = items.full_iter_mut().find(|i| i.data.uid() == new_item.uid()) {
                old_item.data = new_item;
                old_item.is_dirty = true;
            } else {
                items.push(Item::dirty(new_item));
            }
        } else if !is_delete {
            self.table.list.items = Some(FilterableList::from(vec![Item::new(new_item)]));
        }

        self.table.update_data_lengths();
    }
}

impl Responsive for ResourcesList {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.table.process_event(event)
    }
}

impl Table for ResourcesList {
    delegate! {
        to self.table.list {
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn get_filter(&self) -> Option<&str>;
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn get_highlighted_item_uid(&self) -> Option<&str>;
            fn highlight_item_by_name(&mut self, name: &str) -> bool;
            fn highlight_item_by_name_start(&mut self, text: &str) -> bool;
            fn highlight_item_by_uid(&mut self, uid: &str) -> bool;
            fn highlight_item_by_line(&mut self, line_no: u16) -> bool;
            fn highlight_first_item(&mut self) -> bool;
            fn deselect_all(&mut self);
            fn invert_selection(&mut self);
            fn select_highlighted_item(&mut self);
            fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            fn is_anything_selected(&self) -> bool;
            fn update_page(&mut self, new_height: u16);
            fn get_paged_names(&self, width: usize) -> Option<Vec<(String, bool)>>;
        }
    }

    fn clear(&mut self) {
        self.data = InitData::default();
        self.table.list.clear();
    }

    fn filter(&mut self, filter: Option<String>) {
        if self.table.list.filter(filter) {
            self.table.update_data_lengths();
        }
    }

    fn sort(&mut self, column_no: usize, is_descending: bool) {
        self.table.sort(column_no, is_descending);
    }

    fn toggle_sort(&mut self, column_no: usize) {
        self.table.toggle_sort(column_no);
    }

    fn get_sort_symbols(&self) -> Rc<[char]> {
        self.table.header.get_sort_symbols()
    }

    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Option<Vec<(String, TextColors)>> {
        if let Some(list) = self.table.list.get_page() {
            let (namespace_width, name_width, name_extra_width) = self.table.header.get_widths(view, width);

            let mut result = Vec::with_capacity(self.table.list.page_height.into());
            for item in list {
                result.push((
                    item.get_text(
                        view,
                        &self.table.header,
                        width,
                        namespace_width,
                        name_width + name_extra_width,
                    ),
                    item.data.get_colors(theme, item.is_active, item.is_selected),
                ));
            }

            return Some(result);
        }

        None
    }

    fn get_header(&mut self, view: ViewType, width: usize) -> &str {
        self.table.header.get_text(view, width)
    }
}
