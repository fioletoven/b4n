use delegate::delegate;
use std::collections::{HashMap, HashSet};

use crate::ui::{
    ResponseEvent, Responsive, Table, TuiEvent, ViewType,
    colors::TextColors,
    lists::{BasicFilterContext, FilterableList, Item, Row, ScrollableList},
    theme::Theme,
};

use super::KindItem;

type KindFilterableList = FilterableList<Item<KindItem, BasicFilterContext>, BasicFilterContext>;

/// Kubernetes kinds list.
#[derive(Default)]
pub struct KindsList {
    pub list: ScrollableList<KindItem, BasicFilterContext>,
    header: String,
    width: usize,
}

impl KindsList {
    /// Updates [`KindsList`] with new data from [`Vec<KindItem>`].
    pub fn update(&mut self, kinds: Option<Vec<KindItem>>, sort_by: usize, is_descending: bool) {
        if let Some(new_list) = kinds {
            self.list.dirty(false);

            if let Some(old_list) = &mut self.list.items {
                update_old_list(old_list, new_list);
            } else {
                self.list.items = Some(create_new_list(new_list));
            }

            self.list.sort(sort_by, is_descending);
        }
    }

    /// Returns cloned [`KindItem`]s as a vector.
    pub fn to_vec(&self) -> Option<Vec<KindItem>> {
        self.list
            .items
            .as_ref()
            .map(|list| list.full_iter().map(|i| i.data.clone()).collect())
    }
}

impl Responsive for KindsList {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.list.process_event(event)
    }
}

impl Table for KindsList {
    delegate! {
        to self.list {
            fn clear(&mut self);
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn filter(&mut self, filter: Option<String>);
            fn get_filter(&self) -> Option<&str>;
            fn sort(&mut self, column_no: usize, is_descending: bool);
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

    /// Not implemented for [`KindsList`].
    fn toggle_sort(&mut self, _column_no: usize) {
        // pass
    }

    /// Not implemented for [`KindsList`].
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    fn get_header(&mut self, _view: ViewType, width: usize) -> &str {
        if self.width == width {
            return &self.header;
        }

        self.header = format!("{1:<0$}", width, "KIND");
        self.width = width;

        &self.header
    }
}

fn update_old_list(old_list: &mut KindFilterableList, new_list: Vec<KindItem>) {
    let mut unique = HashSet::new();
    let mut multiple = HashSet::new();

    for new_item in new_list {
        let name = new_item.name.clone();
        if unique.contains(&name) {
            multiple.insert(name);
        } else {
            unique.insert(name);
        }

        let old_item = old_list.full_iter_mut().find(|i| i.data.uid() == new_item.uid());
        if let Some(old_item) = old_item {
            old_item.data = new_item;
            old_item.is_dirty = true;
        } else {
            old_list.push(Item::dirty(new_item));
        }
    }

    old_list.full_retain(|i| i.is_dirty || i.is_fixed);

    mark_multiple(old_list, &multiple);
}

fn create_new_list(new_list: Vec<KindItem>) -> KindFilterableList {
    let mut unique = HashSet::new();
    let mut multiple = HashSet::new();

    let mut list = Vec::with_capacity(new_list.len());

    for new_item in new_list {
        let name = new_item.name.clone();
        if unique.contains(&name) {
            multiple.insert(name);
        } else {
            unique.insert(name);
        }

        list.push(Item::new(new_item));
    }

    let mut list = FilterableList::from(list);

    mark_multiple(&mut list, &multiple);

    list
}

fn mark_multiple(list: &mut KindFilterableList, multiple: &HashSet<String>) {
    for item in list.full_iter_mut() {
        item.data.multiple = multiple.contains(item.data.name());
    }
}
