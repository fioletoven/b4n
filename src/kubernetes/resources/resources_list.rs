use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use delegate::delegate;
use std::{collections::HashMap, rc::Rc};

use crate::{
    core::{InitData, ObserverResult},
    kubernetes::{
        ALL_NAMESPACES, NAMESPACES, Namespace,
        resources::{CONTAINERS, ResourceFilterContext, ResourceItem},
    },
    ui::{
        ResponseEvent, Responsive, Table, ViewType,
        colors::TextColors,
        lists::{FilterableList, Header, Item, Row, ScrollableList},
        theme::Theme,
    },
};

/// Kubernetes resources list.
#[derive(Default)]
pub struct ResourcesList {
    pub data: InitData,
    pub list: ScrollableList<ResourceItem, ResourceFilterContext>,
    header: Header,
}

impl ResourcesList {
    /// Sets filter settings for [`ResourcesList`].
    pub fn with_filter_settings(mut self, settings: Option<impl Into<String>>) -> Self {
        self.list.set_filter_settings(settings);
        self
    }

    /// Updates [`ResourcesList`] with new data from [`ObserverResult`] and sorts the new list if needed.\
    /// Returns `true` if the kind was changed during the update.
    pub fn update(&mut self, result: ObserverResult) -> bool {
        let (mut sort_by, mut is_descending) = self.header.sort_info();
        match result {
            ObserverResult::Init(init) => {
                self.update_kind(init);
                (sort_by, is_descending) = self.header.sort_info();
                self.header.set_sort_info(sort_by, is_descending);
                true
            },
            ObserverResult::InitDone => false,
            ObserverResult::Apply(resource) => {
                self.update_list(resource, false);
                self.sort_internal_list(sort_by, is_descending);
                false
            },
            ObserverResult::Delete(resource) => {
                self.update_list(resource, true);
                self.sort_internal_list(sort_by, is_descending);
                false
            },
        }
    }

    /// Returns `true` if the resources in the list are of a special type `containers`.
    pub fn has_containers(&self) -> bool {
        self.data.kind_plural == CONTAINERS
    }

    /// Gets highlighted resource.
    pub fn get_highlighted_resource(&self) -> Option<&ResourceItem> {
        self.list.get_highlighted_item().map(|i| &i.data)
    }

    /// Gets specific resource.
    pub fn get_resource(&self, name: &str, namespace: &Namespace) -> Option<&ResourceItem> {
        self.list.items.as_ref().and_then(|items| {
            items
                .full_iter()
                .find(|i| i.data.name == name && i.data.namespace.as_deref() == namespace.as_option())
                .map(|i| &i.data)
        })
    }

    fn update_kind(&mut self, init: InitData) {
        self.data = init;
        self.header = ResourceItem::header(&self.data.kind);
        self.list.clear();
        if self.data.kind_plural == NAMESPACES {
            self.list.items = Some(FilterableList::from(vec![Item::fixed(ResourceItem::new(ALL_NAMESPACES))]));
        }
    }

    /// Adds, updates or deletes `new_item` from the resources list.
    fn update_list(&mut self, new_item: ResourceItem, is_delete: bool) {
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
                }

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
        self.data = InitData::default();
        self.list.clear();
    }

    fn filter(&mut self, filter: Option<String>) {
        if self.list.filter(filter) {
            self.update_data_lengths();
        }
    }

    fn sort(&mut self, column_no: usize, is_descending: bool) {
        if column_no < self.header.get_columns_count() {
            self.header.set_sort_info(column_no, is_descending);
            self.sort_internal_list(column_no, is_descending);
        }
    }

    fn get_sort_symbols(&self) -> Rc<[char]> {
        self.header.get_sort_symbols()
    }

    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Option<Vec<(String, TextColors)>> {
        if let Some(list) = self.list.get_page() {
            let (namespace_width, name_width, name_extra_width) = self.header.get_widths(view, width);

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
        self.header.get_text(view, width)
    }
}
