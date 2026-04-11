use b4n_config::themes::{TextColors, Theme};
use b4n_kube::{ALL_NAMESPACES, CONTAINERS, NAMESPACES, Namespace, Scope};
use b4n_kube::{InitData, ObserverResult};
use b4n_list::{Item, Row, ScrollableList};
use b4n_tui::table::{ItemExt, TabularList, ViewType};
use b4n_tui::widgets::ActionItem;
use b4n_tui::{ResponseEvent, Responsive, TuiEvent, table::Table};
use delegate::delegate;
use std::time::{Duration, Instant};
use std::{collections::HashMap, rc::Rc};

use crate::kube::resources::{ResourceFilterContext, ResourceItem};

static CACHE_EXPIRED_DURATION: Duration = Duration::from_secs(120);

/// Kubernetes resources list.
#[derive(Default)]
pub struct ResourcesList {
    pub data: InitData,
    pub table: TabularList<ResourceItem, ResourceFilterContext>,
    cache: HashMap<String, CacheEntry>,
    is_from_cache: bool,
    last_cache_cleanup: Option<Instant>,
}

impl ResourcesList {
    /// Sets filter settings for [`ResourcesList`].
    pub fn with_filter_settings(mut self, settings: Option<impl Into<String>>) -> Self {
        self.table.list.set_filter_settings(settings);
        self
    }

    /// Tries to restore list from the cache.
    pub fn restore_from_cache(&mut self, key: &str) -> bool {
        if let Some(mut entry) = self.cache.remove(key)
            && !entry.is_expired()
        {
            self.update_kind(entry.init, true);
            for item in entry.list.full_iter_mut() {
                item.data.is_cached = !item.is_fixed;
                item.is_selected = false;
            }

            self.is_from_cache = true;
            self.table.list = entry.list;
            self.table.update_data_lengths();

            return true;
        }

        false
    }

    /// Updates [`ResourcesList`] with new data from [`ObserverResult`] and sorts the new list if needed.\
    /// Returns `true` if the kind was changed during the update.
    pub fn update(&mut self, result: ObserverResult<ResourceItem>) -> bool {
        let (sort_by, is_descending) = self.table.header.sort_info();
        match result {
            ObserverResult::Init(init) => {
                self.update_kind(*init, self.is_from_cache);
                let (sort_by, is_descending) = self.table.header.sort_info();
                self.sort(sort_by, is_descending);
                self.is_from_cache = false;
                true
            },
            ObserverResult::InitDone => {
                self.table.list.full_retain(|i| !i.data.is_cached);
                false
            },
            ObserverResult::Apply(resource) => {
                self.add_all_namespaces_item();
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

    /// Removes all expired entries from the cache, freeing their associated memory.
    pub fn remove_expired_cache_entries(&mut self) {
        if self.last_cache_cleanup.is_none_or(|f| f.elapsed() >= Duration::from_secs(1)) {
            self.last_cache_cleanup = Some(Instant::now());
            self.cache.retain(|_, v| !v.is_expired());
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

    /// Returns `true` if the item with specified `name` and `group` was selected on the list.\
    /// **Note** that if `group` is empty it is omitted during check.
    pub fn highlight_item_by_name_and_group(&mut self, name: &str, group: &str) -> bool {
        if group.is_empty() {
            self.table.list.highlight_item_by_name(name)
        } else {
            self.table
                .list
                .highlight_item_by(|i| i.data.name() == name && i.data.group() == group)
        }
    }

    /// Gets highlighted item `name` and `group`.
    pub fn get_highlighted_item_name_and_group(&self) -> Option<(&str, &str)> {
        self.table
            .list
            .get_highlighted_item()
            .map(|i| (i.data.name(), i.data.group()))
    }

    /// Gets highlighted resource.
    pub fn get_highlighted_resource(&self) -> Option<&ResourceItem> {
        self.table.list.get_highlighted_item().map(|i| &i.data)
    }

    /// Gets specific resource.
    pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&ResourceItem> {
        self.table
            .list
            .full_iter()
            .find(|i| i.data.name == name && i.data.namespace.as_deref() == namespace.as_option())
            .map(|i| &i.data)
    }

    /// Gets selected resources.
    pub fn get_selected_resources(&self) -> Vec<&ResourceItem> {
        self.table
            .list
            .iter()
            .filter(|i| i.is_selected)
            .map(|i| &i.data)
            .collect::<Vec<_>>()
    }

    /// Returns resources as a list of formatted strings.\
    /// **Note** that this is the same format as for drawing on the terminal.
    pub fn get_items_as_text(&mut self, view: ViewType, selected: bool) -> Vec<String> {
        let width = self.get_best_width(view);
        let header = self.table.header.get_text(view, width).to_string();
        let (namespace_width, name_width, name_extra_width) = self.table.header.get_widths(view, width);
        let name_width = name_width + name_extra_width;

        let items = self
            .table
            .list
            .iter()
            .filter(|i| !selected || i.is_selected)
            .collect::<Vec<_>>();

        if items.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(items.len() + 1);
        result.push(header);
        for item in items {
            result.push(item.get_text(view, &self.table.header, width, namespace_width, name_width, 0));
        }

        result
    }

    /// Returns resources names as a list.
    pub fn get_names(&self) -> Vec<String> {
        self.table.list.full_iter().map(|i| i.data.name.clone()).collect()
    }

    /// Sorts items in the list again, using the same settings as last sort.
    pub fn resort(&mut self) {
        let (sort_by, is_descending) = self.table.header.sort_info();
        self.sort(sort_by, is_descending);
    }

    fn get_best_width(&self, view: ViewType) -> usize {
        let group_width = if view == ViewType::Full {
            self.table.header.get_data_length(0).max(9) + 2
        } else {
            0
        };
        let name_width = self.table.header.get_data_length(1).max(4) + 2;
        let extra_width = self
            .table
            .header
            .get_extra_columns()
            .map(|c| c.iter().map(|c| c.len() + 2).sum::<usize>())
            .unwrap_or_default();
        let age_width = self
            .table
            .header
            .get_data_length(self.table.header.get_columns_count() - 1)
            .max(7);

        group_width + name_width + extra_width + age_width
    }

    fn update_kind(&mut self, init: InitData, is_from_cache: bool) {
        let are_equal = self.data.resource.is_equal(&init.resource, &init.scope);
        self.data = init;
        if !is_from_cache || !are_equal {
            self.table.update_header(ResourceItem::header(
                &self.data.kind,
                &self.data.group,
                self.data.crd.as_ref(),
                self.data.has_metrics,
                self.data.resource.is_filtered(),
            ));
        }
    }

    /// Adds, updates or deletes `new_item` from the resources list.
    fn update_list(&mut self, new_item: ResourceItem, is_delete: bool) {
        if is_delete {
            let index = self.table.list.full_iter().position(|i| i.data.uid() == new_item.uid());
            if let Some(index) = index {
                self.table.list.full_remove(index);
            }
        } else if let Some(old_item) = self.table.list.full_iter_mut().find(|i| i.data.uid() == new_item.uid()) {
            old_item.data = new_item;
            old_item.is_dirty = true;
        } else {
            self.table.list.push(Item::dirty(new_item));
        }

        self.table.update_data_lengths();
    }

    fn add_all_namespaces_item(&mut self) {
        if self.table.list.full_len() == 0 && self.data.kind_plural == NAMESPACES {
            self.table.list.push(Item::fixed(ResourceItem::new(ALL_NAMESPACES, true)));
        }
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
            fn filter(&self) -> Option<&str>;
            fn is_anything_highlighted(&self) -> bool;
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn get_highlighted_item_uid(&self) -> Option<&str>;
            fn get_highlighted_item_line_no(&self) -> Option<u16>;
            fn highlight_item_by_name(&mut self, name: &str) -> bool;
            fn highlight_item_by_name_start(&mut self, text: &str) -> bool;
            fn highlight_item_by_uid(&mut self, uid: &str) -> bool;
            fn highlight_item_by_line(&mut self, line_no: u16) -> bool;
            fn highlight_first_item(&mut self) -> bool;
            fn unhighlight_item(&mut self);
            fn select_all(&mut self);
            fn deselect_all(&mut self);
            fn invert_selection(&mut self);
            fn select_highlighted_item(&mut self);
            fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            fn is_anything_selected(&self) -> bool;
            fn update_page(&mut self, new_height: u16);
            fn get_paged_names(&self, width: usize) -> Vec<(String, bool)>;
        }
    }

    /// Clears the list, moving to the cache all values.
    fn clear(&mut self) {
        self.is_from_cache = false;

        let data = std::mem::take(&mut self.data);
        let list = std::mem::take(&mut self.table.list);
        self.table.list.set_filter_settings(list.filter_settings());

        if data.resource.kind.as_str().is_empty() {
            return;
        }

        if data.scope == Scope::Namespaced {
            let key = if data.resource.is_container()
                && let Some(name) = &data.resource.name
            {
                format!("{}/pods/{}", data.resource.namespace.as_str(), name)
            } else {
                format!("{}/{}", data.resource.namespace.as_str(), data.resource.kind.as_str())
            };
            self.cache.insert(key, CacheEntry::new(data, list));
        } else {
            self.cache
                .insert(data.resource.kind.as_str().to_owned(), CacheEntry::new(data, list));
        }
    }

    fn set_filter(&mut self, filter: Option<String>) {
        if self.table.list.set_filter(filter) {
            self.table.update_data_lengths();
        }
    }

    fn get_column_at_position(&self, position: usize) -> Option<usize> {
        self.table.get_column_at_position(position)
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

    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Vec<(String, TextColors)> {
        let (namespace_width, name_width, name_extra_width) = self.table.header.get_widths(view, width);

        let mut result = Vec::with_capacity(self.table.list.page_height().into());
        for item in self.table.list.get_page() {
            result.push((
                item.get_text(
                    view,
                    &self.table.header,
                    width,
                    namespace_width,
                    name_width + name_extra_width,
                    self.table.offset(),
                ),
                item.data.get_colors(theme, item.is_active, item.is_selected),
            ));
        }

        result
    }

    fn get_header(&mut self, view: ViewType, width: usize) -> &str {
        self.table.header.get_text(view, width)
    }

    fn refresh_header(&mut self, view: ViewType, width: usize) {
        self.table.header.refresh_text(view, width);
    }

    fn offset(&self) -> usize {
        self.table.offset()
    }

    fn refresh_offset(&mut self) -> usize {
        self.table.get_offset()
    }
}

impl From<&ResourceItem> for ActionItem {
    fn from(value: &ResourceItem) -> Self {
        ActionItem::raw(value.uid.clone(), "resource".to_owned(), value.name.clone(), None)
    }
}

struct CacheEntry {
    time: Instant,
    init: InitData,
    list: ScrollableList<ResourceItem, ResourceFilterContext>,
}

impl CacheEntry {
    fn new(init: InitData, list: ScrollableList<ResourceItem, ResourceFilterContext>) -> Self {
        Self {
            time: Instant::now(),
            init,
            list,
        }
    }

    fn is_expired(&self) -> bool {
        self.time.elapsed() > CACHE_EXPIRED_DURATION
    }
}
