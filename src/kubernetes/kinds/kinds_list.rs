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
    for new_item in new_list {
        let old_item = old_list.full_iter_mut().find(|i| i.data.uid() == new_item.uid());
        if let Some(old_item) = old_item {
            old_item.data = new_item;
            old_item.is_dirty = true;
        } else {
            old_list.push(Item::dirty(new_item));
        }
    }

    old_list.full_retain(|i| i.is_dirty || i.is_fixed);

    recalculate_multiple_flags(old_list);
}

fn create_new_list(new_list: Vec<KindItem>) -> KindFilterableList {
    let mut list = FilterableList::from(new_list.into_iter().map(Item::new).collect());
    recalculate_multiple_flags(&mut list);
    list
}

fn recalculate_multiple_flags(list: &mut KindFilterableList) {
    let mut unique_name = HashSet::with_capacity(list.full_len());
    let mut unique_group = HashSet::with_capacity(list.full_len());
    let mut multiple_groups = HashSet::with_capacity(list.full_len());
    let mut multiple_versions = HashSet::with_capacity(list.full_len());

    for item in list.full_iter() {
        let name = item.data.kind.name().to_owned();
        let group = item.data.kind.name_and_group().to_owned();

        if unique_name.contains(&name) {
            multiple_groups.insert(name);
        } else {
            unique_name.insert(name);
        }

        if item.data.kind.has_group() {
            if unique_group.contains(&group) {
                multiple_versions.insert(group);
            } else {
                unique_group.insert(group);
            }
        }
    }

    for item in list.full_iter_mut() {
        item.data.multiple_groups = multiple_groups.contains(item.data.name());
        item.data.multiple_versions = multiple_versions.contains(item.data.kind.name_and_group());
    }
}
