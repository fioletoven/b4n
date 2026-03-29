use b4n_config::HistoryItem;
use b4n_list::{BasicFilterContext, Item, ScrollableList};
use b4n_tui::{ResponseEvent, Responsive, TuiEvent, table::Table};
use delegate::delegate;
use std::collections::HashMap;

use super::PatternItem;

/// Filter patterns list.
#[derive(Default)]
pub struct PatternsList {
    pub list: ScrollableList<PatternItem, BasicFilterContext>,
    description: Option<String>,
}

impl PatternsList {
    /// Creates new [`PatternsList`] instance from the filter history list.
    pub fn from(filter_history: &[HistoryItem], key_name: Option<&str>) -> Self {
        let description = key_name.map(|d| format!("{d} to insert"));
        let mut list = ScrollableList::from(filter_history.iter().map(|p| p.into()).collect::<Vec<_>>());
        list.sort(1, false);
        Self { list, description }
    }

    /// Adds pattern to the list or updates if it already exist.
    pub fn add_or_update(&mut self, pattern: PatternItem) {
        if pattern.value.is_empty() {
            return;
        }

        let position = self.list.full_iter().position(|i| i.data.value == pattern.value);
        if let Some(idx) = position {
            self.list.full_replace(idx, Item::new(pattern));
        } else {
            self.list.push(Item::new(pattern));
        }

        self.list.sort(1, false);
    }

    /// Returns removed item if anything was highlighted.\
    /// Preserves filter and highlights next element from the list.
    pub fn remove_highlighted(&mut self) -> Option<String> {
        if let Some((idx, removed)) = self.remove_highlighted_item() {
            self.list.recover_filter();
            let new_highlight = idx.min(self.list.len().saturating_sub(1));
            if let Some(item) = self.list.iter_mut().nth(new_highlight) {
                item.is_active = true;
            }

            self.list.recover_highlighted_item_index();
            Some(removed)
        } else {
            None
        }
    }

    fn remove_highlighted_item(&mut self) -> Option<(usize, String)> {
        let idx = self.list.iter().position(|i| i.is_active);
        if let Some(idx) = idx {
            let removed = self.list.remove(idx);
            self.list.full_iter_mut().for_each(|i| i.is_active = false);
            Some((idx, removed.data.value))
        } else {
            None
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
            fn set_filter(&mut self, filter: Option<String>);
            fn filter(&self) -> Option<&str>;
            fn sort(&mut self, column_no: usize, is_descending: bool);
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
        }
    }

    fn get_column_at_position(&self, _position: usize) -> Option<usize> {
        None
    }

    /// Not implemented for [`PatternsList`].
    fn toggle_sort(&mut self, _column_no: usize) {
        // pass
    }

    fn get_paged_names(&self, width: usize) -> Vec<(String, bool)> {
        if let Some(description) = &self.description {
            self.list.get_paged_names_with_description(width, description)
        } else {
            self.list.get_paged_names(width)
        }
    }
}
