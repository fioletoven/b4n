use b4n_config::themes::{TextColors, Theme};
use b4n_lists::{BasicFilterContext, FilterableList, ScrollableList};
use delegate::delegate;
use std::collections::HashMap;

use crate::ui::{ResponseEvent, Responsive, Table, TuiEvent, ViewType, lists::ScrollableListExt};

use super::PatternItem;

/// Filter patterns list.
#[derive(Default)]
pub struct PatternsList {
    pub list: ScrollableList<PatternItem, BasicFilterContext>,
    description: Option<String>,
}

impl PatternsList {
    /// Creates new [`PatternsList`] instance from the filter history list.
    pub fn from(filter_history: &[String], key_name: Option<&str>) -> Self {
        let description = key_name.map(|d| format!("{d} to insert"));
        let mut list = ScrollableList::from(filter_history.iter().map(|p| p.as_str().into()).collect());
        list.sort(1, false);
        Self { list, description }
    }

    /// Adds the pattern to the list if it does not already exist. Ensures the list does not exceed `max_list_size`.\
    /// Returns `true` if the pattern was added to the list.
    pub fn add(&mut self, pattern: PatternItem, max_list_size: usize) -> bool {
        if !pattern.value.is_empty() && !self.contains(&pattern) {
            self.list.push(pattern);

            let len = self.list.items.as_ref().map(FilterableList::full_len);
            if len.unwrap_or_default() > max_list_size {
                self.remove_oldest();
            }

            self.list.sort(1, false);

            true
        } else {
            false
        }
    }

    /// Returns [`PatternsList`] as a vector of strings that can be saved in the app history data.
    pub fn to_vec(&self) -> Vec<String> {
        if let Some(list) = &self.list.items {
            list.full_iter().map(|i| i.data.to_string()).collect()
        } else {
            Vec::new()
        }
    }

    /// Returns `true` if the [`PatternsList`] contains an element with the given value.
    fn contains(&self, pattern: &PatternItem) -> bool {
        self.list
            .items
            .as_ref()
            .is_some_and(|l| l.full_iter().any(|i| i.data.value == pattern.value))
    }

    /// Removes the oldest element from a list.
    fn remove_oldest(&mut self) {
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
}

impl Responsive for PatternsList {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.list.process_event(event)
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
        }
    }

    /// Not implemented for [`PatternsList`].
    fn toggle_sort(&mut self, _column_no: usize) {
        // pass
    }

    fn get_paged_names(&self, width: usize) -> Option<Vec<(String, bool)>> {
        if let Some(description) = &self.description {
            self.list.get_paged_names_with_description(width, description)
        } else {
            self.list.get_paged_names(width)
        }
    }

    /// Not implemented for [`PatternsList`].
    fn get_paged_items(&self, _theme: &Theme, _view: ViewType, _width: usize) -> Option<Vec<(String, TextColors)>> {
        None
    }

    /// Returns header text for the list.\
    /// **Note** that this is not implemented for [`PatternsList`].
    fn get_header(&mut self, _view: ViewType, _width: usize) -> &str {
        "n/a"
    }
}
