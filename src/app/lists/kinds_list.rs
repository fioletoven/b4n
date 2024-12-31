use delegate::delegate;
use std::collections::HashMap;

use crate::{
    kubernetes::resources::Kind,
    ui::{colors::TextColors, theme::Theme, ResponseEvent, Responsive, Table, ViewType},
};

use super::{FilterableList, Item, ScrollableList};

/// Kubernetes kinds list
pub struct KindsList {
    pub list: ScrollableList<Kind>,
}

impl KindsList {
    /// Creates new [`KindsList`] instance
    pub fn new() -> Self {
        KindsList {
            list: ScrollableList::new(),
        }
    }

    /// Updates [`KindsList`] with new data from [`Vec<Kind>`].  
    /// Simplified version, takes care only about highlighted item.
    pub fn update(&mut self, kinds: Option<Vec<Kind>>, sort_by: usize, is_descending: bool) {
        if let Some(list) = kinds {
            let highlighted = self.list.get_highlighted_item_name().unwrap_or("").to_owned();
            self.list.items = Some(FilterableList::from(list.into_iter().map(|i| Item::new(i)).collect()));
            self.list.sort(sort_by, is_descending);
            if !highlighted.is_empty() {
                self.list.highlight_item_by_name(&highlighted);
            }
        } else {
            self.list.items = None;
            self.list.highlighted = None;
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
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn filter(&mut self, filter: Option<String>);
            fn get_filter(&self) -> Option<&str>;
            fn sort(&mut self, column_no: usize, is_descending: bool);
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn highlight_item_by_name(&mut self, name: &str);
            fn highlight_item_by_name_start(&mut self, text: &str);
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
