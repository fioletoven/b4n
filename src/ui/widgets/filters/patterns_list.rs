use delegate::delegate;
use std::collections::HashMap;

use crate::ui::{
    ResponseEvent, Responsive, Table, ViewType,
    colors::TextColors,
    lists::{BasicFilterContext, ScrollableList},
    theme::Theme,
};

use super::PatternItem;

/// Filter patterns list.
#[derive(Default)]
pub struct PatternsList {
    pub list: ScrollableList<PatternItem, BasicFilterContext>,
}

impl PatternsList {
    /// Creates new [`PatternsList`] instance from the filter history list.
    pub fn from(filter_history: Vec<String>) -> Self {
        let mut list = ScrollableList::from(filter_history.into_iter().map(PatternItem::from).collect());
        list.sort(1, false);
        Self { list }
    }

    /// Returns `true` if the [`PatternsList`] contains an element with the given value.
    pub fn contains(&self, value: &str) -> bool {
        self.list
            .items
            .as_ref()
            .is_some_and(|l| l.full_iter().any(|i| i.data.value == value))
    }

    /// Removes the oldest element from a list.
    pub fn remove_oldest(&mut self) {
        if let Some(list) = &mut self.list.items {
            let index = list
                .full_iter()
                .enumerate()
                .min_by_key(|(_, i)| i.data.creation_time)
                .map(|(index, _)| index);
            if let Some(index) = index {
                list.full_remove(index);
            }
        }
    }

    /// Returns [`PatternsList`] as a filter history that can be saved in the app history data.
    pub fn get_filter_history(&self) -> Vec<String> {
        if let Some(list) = &self.list.items {
            list.full_iter().map(|i| i.data.to_string()).collect()
        } else {
            Vec::new()
        }
    }
}

impl Responsive for PatternsList {
    fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        self.list.process_key(key)
    }
}

impl Table for PatternsList {
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
    /// **Note** that this is not implemented for [`PatternsList`].
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    /// Returns header text for the list.  
    /// **Note** that this is not implemented for [`PatternsList`].
    fn get_header(&mut self, _view: ViewType, _width: usize) -> &str {
        "n/a"
    }
}
