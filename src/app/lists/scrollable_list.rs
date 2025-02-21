use crossterm::event::{KeyCode, KeyEvent};
use std::{cmp::Ordering, collections::HashMap};

use crate::ui::ResponseEvent;

use super::{FilterableList, Item, Row};

/// Scrollable UI list.
pub struct ScrollableList<T: Row> {
    pub items: Option<FilterableList<Item<T>>>,
    pub highlighted: Option<usize>,
    pub page_start: usize,
    pub page_height: u16,
    filter: Option<String>,
    is_wide_filter_enabled: bool,
}

impl<T: Row> Default for ScrollableList<T> {
    fn default() -> Self {
        ScrollableList {
            items: None,
            highlighted: None,
            page_start: 0,
            page_height: 0,
            filter: None,
            is_wide_filter_enabled: false,
        }
    }
}

impl<T: Row> ScrollableList<T> {
    /// Creates new [`ScrollableList`] with initial fixed items.
    pub fn fixed(items: Vec<T>) -> Self {
        let list = items.into_iter().map(Item::fixed).collect::<Vec<_>>();

        ScrollableList {
            items: Some(FilterableList::from(list)),
            ..Default::default()
        }
    }

    /// Creates new [`ScrollableList`] with initial items.
    pub fn from(items: Vec<T>) -> Self {
        let list = items.into_iter().map(Item::new).collect::<Vec<_>>();

        ScrollableList {
            items: Some(FilterableList::from(list)),
            ..Default::default()
        }
    }

    /// Clears the [`ScrollableList`], removing all values.
    pub fn clear(&mut self) {
        if let Some(items) = &mut self.items {
            items.clear();
        }
    }

    /// Returns the number of elements in the scrollable list.
    pub fn len(&self) -> usize {
        self.items.as_ref().map(|l| l.len()).unwrap_or_default()
    }

    /// Returns `true` if the scrollable list contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets value of the property `dirty` for all items in the list to `is_dirty`.
    pub fn dirty(&mut self, is_dirty: bool) {
        if let Some(list) = &mut self.items {
            for item in list.full_iter_mut() {
                item.is_dirty = is_dirty;
            }
        }
    }

    /// Sorts items in the list by column number.
    pub fn sort(&mut self, column_no: usize, is_descending: bool) {
        if let Some(items) = &mut self.items {
            if is_descending {
                items.full_sort_by(|a, b| cmp(b, a, column_no));
            } else {
                items.full_sort_by(|a, b| cmp(a, b, column_no));
            }
        }

        if self.items.is_some() {
            self.apply_filter();
            self.highlighted = self.recover_highlighted_item_index();
        } else {
            self.highlighted = None;
        }
    }

    /// Returns `true` if list is filtered.
    pub fn is_filtered(&self) -> bool {
        self.filter.is_some()
    }

    /// Filters items in the list by calling `contains` on each [`Row`].
    pub fn filter(&mut self, filter: Option<String>) {
        self.filter = filter;
        if self.filter.is_some() {
            self.deselect_all();
            self.apply_filter();
        } else if let Some(list) = &mut self.items {
            list.filter_reset();
        }

        self.highlighted = self.recover_highlighted_item_index();
        if let Some(list) = &mut self.items {
            list.full_iter_mut().for_each(|i| i.is_active = false);
            if let Some(highlighted) = self.highlighted {
                list[highlighted].is_active = true;
            }
        }
    }

    /// Returns currently applied filter value.
    pub fn get_filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    /// Sets wide filter to `is_enabled`.
    pub fn set_wide_filter(&mut self, is_enabled: bool) {
        self.is_wide_filter_enabled = is_enabled;
    }

    /// Process [`KeyEvent`] to move over the list.
    pub fn process_key(&mut self, key: KeyEvent) -> ResponseEvent {
        match key.code {
            KeyCode::Home => self.move_highlighted(i32::MIN),
            KeyCode::Up => self.move_highlighted(-1),
            KeyCode::PageUp => self.move_highlighted(-i32::from(self.page_height)),
            KeyCode::Down => self.move_highlighted(1),
            KeyCode::PageDown => self.move_highlighted(i32::from(self.page_height)),
            KeyCode::End => self.move_highlighted(i32::MAX),
            _ => return ResponseEvent::NotHandled,
        }

        ResponseEvent::Handled
    }

    /// Updates page start for the current page size and highlighted resource item.
    pub fn update_page(&mut self, new_height: u16) {
        self.page_height = new_height;
        let highlighted_item = self.highlighted.unwrap_or(0);

        if self.page_start >= highlighted_item {
            self.page_start = highlighted_item;
        } else if self.page_start + usize::from(self.page_height) - 1 < highlighted_item {
            self.page_start = highlighted_item - usize::from(self.page_height) + 1;
        }

        if let Some(items) = &self.items {
            if items.len() < usize::from(self.page_height) {
                self.page_start = 0;
            } else if items.len() < self.page_start + usize::from(self.page_height) {
                self.page_start = items.len() - usize::from(self.page_height);
            }
        }
    }

    /// Returns list items iterator for the current page.
    pub fn get_page(&self) -> Option<impl Iterator<Item = &Item<T>>> {
        self.items
            .as_ref()
            .map(|list| list.iter().skip(self.page_start).take(self.page_height.into()))
    }

    /// Removes all fixed items from the list.
    pub fn remove_fixed(&mut self) {
        if let Some(items) = &mut self.items {
            items.full_retain(|item| !item.is_fixed);
            self.apply_filter();
        }
    }

    /// Clears items selection.
    pub fn deselect_all(&mut self) {
        if let Some(items) = &mut self.items {
            items.iter_mut().for_each(|item| item.is_selected = false);
        }
    }

    /// Inverts selection of items in list.
    pub fn invert_selection(&mut self) {
        if let Some(items) = &mut self.items {
            items.iter_mut().for_each(|item| item.is_selected = !item.is_selected);
        }
    }

    /// Selects / deselects currently highlighted item.
    pub fn select_highlighted_item(&mut self) {
        if let Some(items) = &mut self.items {
            if let Some(highlighted) = self.highlighted {
                if highlighted < items.len() {
                    items[highlighted].is_selected = !items[highlighted].is_selected;
                }
            }
        }
    }

    /// Returns selected item names grouped in [`HashMap`].
    pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>> {
        if let Some(items) = &self.items {
            let mut result: HashMap<&str, Vec<&str>> = HashMap::new();
            for item in items {
                if !item.is_selected {
                    continue;
                }

                if result.contains_key(item.data.group()) {
                    result.get_mut(item.data.group()).unwrap().push(item.data.name());
                } else {
                    result.insert(item.data.group(), vec![item.data.name()]);
                }
            }

            result
        } else {
            HashMap::new()
        }
    }

    /// Returns `true` if anything is selected.
    pub fn is_anything_selected(&self) -> bool {
        if let Some(items) = &self.items {
            return items.iter().any(|i| i.is_selected);
        }

        false
    }

    /// Gets highlighted element index.
    pub fn get_highlighted_item_index(&self) -> Option<usize> {
        self.highlighted
    }

    /// Gets highlighted element name.
    pub fn get_highlighted_item_name(&self) -> Option<&str> {
        self.get_highlighted_item().map(|i| i.data.name())
    }

    /// Gets highlighted element.
    pub fn get_highlighted_item(&self) -> Option<&Item<T>> {
        if let Some(items) = &self.items {
            if let Some(highlighted) = self.highlighted {
                if highlighted < items.len() {
                    return Some(&items[highlighted]);
                }
            }
        }

        None
    }

    /// Gets the highlighted item index from the `is_active` property.
    pub fn recover_highlighted_item_index(&self) -> Option<usize> {
        if let Some(items) = &self.items {
            items.iter().position(|i| i.is_active)
        } else {
            None
        }
    }

    /// Highlights element on list by its name.
    pub fn highlight_item_by_name(&mut self, name: &str) -> bool {
        self.highlight_item_by(|i| i.data.is_equal(name))
    }

    /// Highlights first element on the list which name starts with `text`.
    pub fn highlight_item_by_name_start(&mut self, text: &str) -> bool {
        self.highlight_item_by(|i| i.data.starts_with(text))
    }

    /// Highlights first item on the list, returns `true` on success.
    pub fn highlight_first_item(&mut self) -> bool {
        let Some(items) = &mut self.items else {
            return false;
        };
        if items.is_empty() {
            return false;
        }

        if let Some(highlighted) = self.highlighted {
            if highlighted < items.len() {
                items[highlighted].is_active = false;
            }
        }

        items[0].is_active = true;
        self.highlighted = Some(0);
        true
    }

    /// Returns item names from the current page together with indication if item is active.
    pub fn get_paged_names(&self, width: usize) -> Option<Vec<(String, bool)>> {
        if let Some(list) = self.get_page() {
            let mut result = Vec::with_capacity(self.page_height.into());
            for item in list {
                result.push((item.data.get_name(width), item.is_active));
            }

            return Some(result);
        }

        None
    }

    /// Tries to highlight item finding it by closure.
    fn highlight_item_by<F>(&mut self, f: F) -> bool
    where
        F: Fn(&Item<T>) -> bool,
    {
        if let Some(items) = &mut self.items {
            let maybe_index = items.iter().position(f);
            if let Some(index) = maybe_index {
                if let Some(highlighted) = self.highlighted {
                    if highlighted < items.len() {
                        items[highlighted].is_active = false;
                    }
                }

                items[index].is_active = true;
                self.highlighted = Some(index);

                return true;
            }
        }

        false
    }

    /// Adds `rows_to_move` to the currently highlighted item index.
    fn move_highlighted(&mut self, rows_to_move: i32) {
        if let Some(items) = &mut self.items {
            if items.is_empty() || rows_to_move == 0 {
                return;
            }

            if self.highlighted.is_none() && rows_to_move == 1 {
                items[0].is_active = true;
                self.highlighted = Some(0);
            } else {
                let highlighted = self.highlighted.unwrap_or(0);
                let new_highlighted = std::cmp::max(highlighted as isize + rows_to_move as isize, 0) as usize;
                let new_highlighted = std::cmp::min(new_highlighted, items.len() - 1);

                items[highlighted].is_active = false;
                items[new_highlighted].is_active = true;
                self.highlighted = Some(new_highlighted);
            }
        }
    }

    /// Re-applies remembered text filter to the list.
    fn apply_filter(&mut self) {
        if let Some(list) = &mut self.items {
            if let Some(filter) = &self.filter {
                if self.is_wide_filter_enabled {
                    list.filter(|x| x.data.wide_contains(filter));
                } else {
                    list.filter(|x| x.data.contains(filter));
                }
            }
        }
    }
}

/// Compares two [`Item`]s by selected column values ignoring fixed items.
fn cmp(a: &Item<impl Row>, b: &Item<impl Row>, column: usize) -> Ordering {
    if a.is_fixed || b.is_fixed {
        return Ordering::Equal;
    }

    a.data.column_text(column).cmp(b.data.column_text(column))
}
