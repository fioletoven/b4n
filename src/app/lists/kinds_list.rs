use delegate::delegate;
use std::collections::{HashMap, HashSet};

use crate::{
    kubernetes::resources::Kind,
    ui::{ResponseEvent, Responsive, Table, ViewType, colors::TextColors, theme::Theme},
};

use super::{BasicFilterContext, FilterableList, Item, Row, ScrollableList};

type KindFilterableList = FilterableList<Item<Kind, BasicFilterContext>, BasicFilterContext>;

/// Kubernetes kinds list.
#[derive(Default)]
pub struct KindsList {
    pub list: ScrollableList<Kind, BasicFilterContext>,
}

impl KindsList {
    /// Updates [`KindsList`] with new data from [`Vec<Kind>`].
    pub fn update(&mut self, kinds: Option<Vec<Kind>>, sort_by: usize, is_descending: bool) {
        if let Some(new_list) = kinds {
            self.list.dirty(false);

            if let Some(old_list) = &mut self.list.items {
                update_old_list(old_list, new_list);
            } else {
                self.list.items = create_new_list(new_list);
            }

            self.list.sort(sort_by, is_descending);
        }
    }
}

impl Responsive for KindsList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.list.process_key(key)
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

    /// Returns items from the current page in a form of text lines to display and colors for that lines.  
    /// As kinds are used only in selector, we don't care to implement this.
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    fn get_header(&self, _view: ViewType, width: usize) -> String {
        format!("{1:<0$}", width, "KIND")
    }
}

fn update_old_list(old_list: &mut KindFilterableList, new_list: Vec<Kind>) {
    let mut unique = HashSet::new();
    let mut multiple = HashSet::new();

    for new_item in new_list.into_iter() {
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

fn create_new_list(new_list: Vec<Kind>) -> Option<KindFilterableList> {
    let mut unique = HashSet::new();
    let mut multiple = HashSet::new();

    let mut list = Vec::with_capacity(new_list.len());

    for new_item in new_list.into_iter() {
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

    Some(list)
}

fn mark_multiple(list: &mut KindFilterableList, multiple: &HashSet<String>) {
    for item in list.full_iter_mut() {
        item.data.multiple = multiple.contains(item.data.name());
    }
}
