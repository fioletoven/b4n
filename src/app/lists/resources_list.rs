use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use delegate::delegate;
use kube::discovery::Scope;
use std::{collections::HashMap, rc::Rc};

use crate::{
    app::{ObserverInitData, ObserverResult},
    kubernetes::{
        ALL_NAMESPACES, NAMESPACES, Namespace,
        resources::{Resource, ResourceFilterContext},
    },
    ui::{ResponseEvent, Responsive, Table, ViewType, colors::TextColors, theme::Theme},
};

use super::{FilterableList, Header, Item, Row, ScrollableList};

/// Kubernetes resources list.
pub struct ResourcesList {
    pub kind: String,
    pub kind_plural: String,
    pub group: String,
    pub scope: Scope,
    pub list: ScrollableList<Resource, ResourceFilterContext>,
    header: Header,
    header_cache: HeaderCache,
}

impl Default for ResourcesList {
    fn default() -> Self {
        ResourcesList {
            kind: String::new(),
            kind_plural: String::new(),
            group: String::new(),
            scope: Scope::Cluster,
            list: ScrollableList::default(),
            header: Header::default(),
            header_cache: HeaderCache::default(),
        }
    }
}

impl ResourcesList {
    /// Sets filter settings for [`ResourcesList`].
    pub fn with_filter_settings(mut self, settings: Option<impl Into<String>>) -> Self {
        self.list.set_filter_settings(settings);
        self
    }

    /// Updates [`ResourcesList`] with new data from [`ObserverResult`] and sorts the new list if needed.  
    /// Returns `true` if the kind was also changed during the update.
    pub fn update(&mut self, result: Box<ObserverResult>) -> bool {
        let (mut sort_by, mut is_descending) = self.header.sort_info();
        if self.update_kind(result.init) {
            (sort_by, is_descending) = self.header.sort_info();
            self.header.set_sort_info(sort_by, is_descending);
            self.header_cache.invalidate();
            true
        } else {
            if let Some(resource) = result.object {
                self.update_list(resource, result.is_delete);
            }
            self.sort_internal_list(sort_by, is_descending);
            false
        }
    }

    /// Gets highlighted resource.
    pub fn get_highlighted_resource(&self) -> Option<&Resource> {
        self.list.get_highlighted_item().map(|i| &i.data)
    }

    /// Gets specific resource.
    pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&Resource> {
        self.list.items.as_ref().and_then(|items| {
            items
                .full_iter()
                .find(|i| i.data.name == name && i.data.namespace.as_deref() == namespace.as_option())
                .map(|i| &i.data)
        })
    }

    /// Gets the widths for namespace and name columns together with extra space for the name column.
    fn get_widths(&self, view: ViewType, width: usize) -> (usize, usize, usize) {
        if view == ViewType::Full {
            self.header.get_full_widths(width)
        } else {
            self.header.get_widths(width)
        }
    }

    /// Returns `true` if kind was changed.
    fn update_kind(&mut self, init: Option<ObserverInitData>) -> bool {
        let Some(init) = init else {
            return false;
        };

        self.kind = init.kind;
        self.kind_plural = init.kind_plural;
        self.group = init.group;
        self.scope = init.scope;
        self.header = Resource::header(&self.kind);
        self.header_cache.invalidate();
        self.list.clear();
        if self.kind_plural == NAMESPACES {
            self.list.items = Some(FilterableList::from(vec![Item::fixed(Resource::new(ALL_NAMESPACES))]));
        }

        true
    }

    /// Adds, updates or deletes `new_item` from the resources list.
    fn update_list(&mut self, new_item: Resource, is_delete: bool) {
        if let Some(items) = &mut self.list.items {
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
            self.list.items = Some(FilterableList::from(vec![Item::new(new_item)]));
        }

        self.update_data_lengths();
    }

    /// Updates max widths for all columns basing on current data in the list.
    fn update_data_lengths(&mut self) {
        self.header.reset_data_lengths();

        let Some(list) = &self.list.items else {
            self.header_cache.invalidate();
            return;
        };

        let columns_no = self.header.get_columns_count();
        for item in list {
            for column in 0..columns_no {
                let column_width = std::cmp::max(
                    self.header.get_data_length(column),
                    item.data.column_text(column).chars().count(),
                );
                self.header.set_data_length(column, column_width);
            }
        }

        self.header.recalculate_extra_columns();
        self.header_cache.invalidate();
    }

    /// Sorts internal resources list.
    fn sort_internal_list(&mut self, column_no: usize, is_descending: bool) {
        let reverse = self.header.has_reversed_order(column_no);
        self.list
            .sort(column_no, if reverse { !is_descending } else { is_descending });
    }
}

impl Responsive for ResourcesList {
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        if key.modifiers == KeyModifiers::ALT && key.code != KeyCode::Char(' ') {
            if let KeyCode::Char(code) = key.code {
                if code.is_numeric() {
                    let (column_no, is_descending) = self.header.sort_info();
                    let sort_by = code.to_digit(10).unwrap() as usize;
                    self.sort(sort_by, if sort_by == column_no { !is_descending } else { false });
                    return ResponseEvent::Handled;
                } else {
                    let sort_symbols = self.header.get_sort_symbols();
                    let uppercase = code.to_ascii_uppercase();
                    let sort_by = sort_symbols.iter().position(|c| *c == uppercase);
                    if let Some(sort_by) = sort_by {
                        let (column_no, is_descending) = self.header.sort_info();
                        self.sort(sort_by, if sort_by == column_no { !is_descending } else { false });
                        return ResponseEvent::Handled;
                    }
                }
            }
        }

        self.list.process_key(key)
    }
}

impl Table for ResourcesList {
    delegate! {
        to self.list {
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn get_filter(&self) -> Option<&str>;
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn highlight_item_by_name(&mut self, name: &str) -> bool;
            fn highlight_item_by_name_start(&mut self, text: &str) -> bool;
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
        self.list.clear();
        self.kind = String::new();
        self.kind_plural = String::new();
        self.group = String::new();
    }

    fn filter(&mut self, filter: Option<String>) {
        if self.list.filter(filter) {
            self.update_data_lengths();
        }
    }

    fn sort(&mut self, column_no: usize, is_descending: bool) {
        if column_no < self.header.get_columns_count() {
            self.header.set_sort_info(column_no, is_descending);
            self.header_cache.invalidate();
            self.sort_internal_list(column_no, is_descending);
        }
    }

    fn get_sort_symbols(&self) -> Rc<[char]> {
        self.header.get_sort_symbols()
    }

    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Option<Vec<(String, TextColors)>> {
        if let Some(list) = self.list.get_page() {
            let (namespace_width, name_width, name_extra_width) = self.get_widths(view, width);

            let mut result = Vec::with_capacity(self.list.page_height.into());
            for item in list {
                result.push((
                    item.data
                        .get_text(view, &self.header, width, namespace_width, name_width + name_extra_width),
                    item.data.get_colors(theme, item.is_active, item.is_selected),
                ));
            }

            return Some(result);
        }

        None
    }

    fn get_header(&mut self, view: ViewType, width: usize) -> &str {
        if self.header_cache.width == width && self.header_cache.view == view {
            return &self.header_cache.text;
        }

        let (namespace_width, name_width, _) = self.get_widths(view, width);
        self.header_cache.text = self.header.get_text(view, namespace_width, name_width, width);
        self.header_cache.view = view;
        self.header_cache.width = width;

        &self.header_cache.text
    }
}

/// Keeps cached header text.
#[derive(Default)]
struct HeaderCache {
    pub text: String,
    pub width: usize,
    pub view: ViewType,
}

impl HeaderCache {
    /// Invalidates cache data.
    pub fn invalidate(&mut self) {
        self.width = 0;
    }
}
