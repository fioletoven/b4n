use delegate::delegate;
use kube::discovery::Scope;
use std::collections::HashMap;

use crate::{
    app::ObserverResult,
    kubernetes::{resources::Resource, ALL_NAMESPACES, NAMESPACES},
    ui::{colors::TextColors, theme::Theme, ResponseEvent, Responsive, Table, ViewType},
};

use super::{FilterableList, Header, Item, Row, ScrollableList};

/// Kubernetes resources list
pub struct ResourcesList {
    pub kind: String,
    pub kind_plural: String,
    pub group: String,
    pub scope: Scope,
    pub header: Header,
    pub list: ScrollableList<Resource>,
}

impl Default for ResourcesList {
    fn default() -> Self {
        ResourcesList {
            kind: String::new(),
            kind_plural: String::new(),
            group: String::new(),
            scope: Scope::Cluster,
            header: Header::default(),
            list: ScrollableList::default(),
        }
    }
}

impl ResourcesList {
    /// Creates new [`ResourcesList`] instance from [`ScrollableList`]
    pub fn from(list: ScrollableList<Resource>) -> Self {
        ResourcesList {
            kind: String::new(),
            kind_plural: String::new(),
            group: String::new(),
            scope: Scope::Cluster,
            header: Header::default(),
            list,
        }
    }

    /// Updates [`ResourcesList`] with new data from [`ObserverResult`] and sorts the new list if needed.  
    /// Returns `true` if the kind was also changed during the update.
    pub fn update(&mut self, result: Option<ObserverResult>, sort_by: usize, is_descending: bool) -> bool {
        if let Some(result) = result {
            let updated = self.update_kind(result.kind, result.kind_plural, result.group, result.scope);
            self.update_list(result.list.iter().map(|r| Resource::from(&self.kind, r)).collect());
            self.list.sort(sort_by, is_descending);

            updated
        } else {
            false
        }
    }

    /// Gets the widths for namespace and name columns together with extra space for the name column
    fn get_widths(&self, view: ViewType, width: usize) -> (usize, usize, usize) {
        if view == ViewType::Full {
            self.header.get_full_widths(width)
        } else {
            self.header.get_widths(width)
        }
    }

    /// Returns `true` if kind was changed
    fn update_kind(&mut self, kind: String, kind_plural: String, group: String, scope: Scope) -> bool {
        if self.kind == kind && self.group == group {
            return false;
        }

        self.kind = kind;
        self.kind_plural = kind_plural;
        self.group = group;
        self.scope = scope.clone();
        self.header = Resource::header(&self.kind);
        self.list.remove_fixed();
        if self.kind_plural == NAMESPACES {
            if let Some(items) = &mut self.list.items {
                items.insert(0, Item::fixed(Resource::new(ALL_NAMESPACES)));
            } else {
                self.list.items = Some(FilterableList::from(vec![Item::fixed(Resource::new(ALL_NAMESPACES))]));
            }
        }

        true
    }

    /// Updates or adds list items from the `new_list`
    fn update_list(&mut self, new_list: Vec<Resource>) {
        self.list.dirty(false);

        if let Some(old_list) = &mut self.list.items {
            for new_item in new_list.into_iter() {
                let old_item = old_list.full_iter_mut().find(|i| i.data.uid() == new_item.uid());
                if let Some(old_item) = old_item {
                    old_item.data = new_item;
                    old_item.is_dirty = true;
                } else {
                    old_list.push(Item::dirty(new_item));
                }
            }

            old_list.full_retain(|i| i.is_dirty || i.is_fixed);
        } else {
            self.list.items = Some(FilterableList::from(new_list.into_iter().map(Item::new).collect()));
        }

        self.update_data_lengths();
    }

    /// Updates max widths for all columns basing on current data in the list
    fn update_data_lengths(&mut self) {
        self.header.reset_data_lengths();
        let Some(list) = &self.list.items else {
            return;
        };

        let columns_no = self.header.get_columns_count();
        for item in list {
            for column in 0..columns_no {
                let column_width = std::cmp::max(self.header.get_data_length(column), item.data.column_text(column).len());
                self.header.set_data_length(column, column_width);
            }
        }

        self.header.recalculate_extra_columns();
    }
}

impl Responsive for ResourcesList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.list.process_key(key)
    }
}

impl Table for ResourcesList {
    delegate! {
        to self.list {
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn filter(&mut self, filter: Option<String>);
            fn get_filter(&self) -> Option<&str>;
            fn sort(&mut self, column_no: usize, is_descending: bool);
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

    fn get_header(&self, view: ViewType, width: usize) -> String {
        let (namespace_width, name_width, _) = self.get_widths(view, width);
        self.header.get_text(view, namespace_width, name_width, width)
    }
}
