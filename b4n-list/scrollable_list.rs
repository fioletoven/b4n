use crossterm::event::KeyCode;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use crate::filter::{FilterableListIterator, FilterableListIteratorMut};
use crate::{FilterContext, FilterData, Filterable, FilterableList, Item, Row};

#[cfg(test)]
#[path = "./scrollable_list.tests.rs"]
mod scrollable_list_tests;

/// Scrollable UI list.
pub struct ScrollableList<T: Row + Filterable<Fc>, Fc: FilterContext> {
    items: FilterableList<Item<T, Fc>, Fc>,
    highlighted: Option<usize>,
    page_start: usize,
    page_height: u16,
    filter: FilterData<Fc>,
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Default for ScrollableList<T, Fc> {
    fn default() -> Self {
        ScrollableList {
            items: FilterableList::default(),
            highlighted: None,
            page_start: 0,
            page_height: 0,
            filter: FilterData::default(),
        }
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> From<Vec<T>> for ScrollableList<T, Fc> {
    fn from(value: Vec<T>) -> Self {
        Self {
            items: value.into_iter().map(Item::new).collect::<Vec<_>>().into(),
            ..Default::default()
        }
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Index<usize> for ScrollableList<T, Fc> {
    type Output = Item<T, Fc>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> IndexMut<usize> for ScrollableList<T, Fc> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.items[index]
    }
}

impl<'a, T: Row + Filterable<Fc>, Fc: FilterContext> IntoIterator for &'a ScrollableList<T, Fc> {
    type Item = &'a Item<T, Fc>;
    type IntoIter = FilterableListIterator<'a, Item<T, Fc>, Fc>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a, T: Row + Filterable<Fc>, Fc: FilterContext> IntoIterator for &'a mut ScrollableList<T, Fc> {
    type Item = &'a mut Item<T, Fc>;
    type IntoIter = FilterableListIteratorMut<'a, Item<T, Fc>, Fc>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Extend<Item<T, Fc>> for ScrollableList<T, Fc> {
    fn extend<I: IntoIterator<Item = Item<T, Fc>>>(&mut self, iter: I) {
        for item in iter {
            self.items.push(item);
        }
        self.apply_filter();
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> ScrollableList<T, Fc> {
    /// Creates new [`ScrollableList`] with initial fixed items.
    pub fn fixed(items: Vec<T>) -> Self {
        Self {
            items: items.into_iter().map(Item::fixed).collect::<Vec<_>>().into(),
            ..Default::default()
        }
    }

    /// Returns the number of elements in the filtered out scrollable list.
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the filtered out scrollable list contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns an iterator over the filtered collection.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Item<T, Fc>> {
        self.items.iter()
    }

    /// Returns a mutable iterator over the filtered collection.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Item<T, Fc>> {
        self.items.iter_mut()
    }

    /// Returns the number of elements in the underneath collection.
    #[inline]
    pub fn full_len(&self) -> usize {
        self.items.full_len()
    }

    /// Returns an iterator over the underneath collection.
    #[inline]
    pub fn full_iter(&self) -> impl Iterator<Item = &Item<T, Fc>> {
        self.items.full_iter()
    }

    /// Returns a mutable iterator over the underneath collection.
    #[inline]
    pub fn full_iter_mut(&mut self) -> impl Iterator<Item = &mut Item<T, Fc>> {
        self.items.full_iter_mut()
    }

    /// Retains only the elements specified by the predicate in the underneath collection.\
    pub fn full_retain<F>(&mut self, f: F)
    where
        F: FnMut(&Item<T, Fc>) -> bool,
    {
        self.items.full_retain(f);
        self.recover_filter();
    }

    /// Removes and returns the element at position `index` within the underneath collection.\
    pub fn full_remove(&mut self, index: usize) -> Item<T, Fc> {
        let result = self.items.full_remove(index);
        self.recover_filter();
        result
    }

    /// Replaces value at position `index`.
    pub fn full_replace(&mut self, index: usize, value: Item<T, Fc>) -> Item<T, Fc> {
        let result = self.items.full_replace(index, value);
        self.recover_filter();
        result
    }

    /// Removes and returns the element at position `index` within the filtered out list.\
    pub fn remove(&mut self, index: usize) -> Item<T, Fc> {
        let result = self.items.remove(index);
        self.recover_filter();
        result
    }

    /// Clears the [`ScrollableList`], removing all values.
    pub fn clear(&mut self) {
        self.items.clear();
        self.filter.set_pattern(None::<String>);
        self.highlighted = None;
        self.page_start = 0;
    }

    /// Replaces all items in the list.\
    /// **Note** that this clears the current filter.
    pub fn set_items(&mut self, items: Vec<Item<T, Fc>>) {
        self.items = items.into();
        self.filter.set_pattern(None::<String>);
        self.highlighted = None;
        self.page_start = 0;
    }

    /// Appends an element to the back of the list.\
    /// **Note** that it may be immediately filtered out by the currently applied filter.
    pub fn push(&mut self, value: Item<T, Fc>) {
        self.items.push(value);
        self.apply_filter();
    }

    /// Sets value of the property `dirty` for all items in the list to `is_dirty`.
    pub fn set_dirty(&mut self, is_dirty: bool) {
        for item in self.items.full_iter_mut() {
            item.is_dirty = is_dirty;
        }
    }

    /// Sorts items in the list by column number.
    pub fn sort(&mut self, column_no: usize, is_descending: bool) {
        if is_descending {
            self.sort_by(|a, b| cmp(b, a, column_no));
        } else {
            self.sort_by(|a, b| cmp(a, b, column_no));
        }
    }

    /// Sorts items in the list with a comparison function.
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Item<T, Fc>, &Item<T, Fc>) -> Ordering,
    {
        self.items.full_sort_by(compare);
        self.apply_filter();
        self.recover_highlighted_item_index();
    }

    /// Returns `true` if list is filtered.
    pub fn is_filtered(&self) -> bool {
        self.filter.has_pattern()
    }

    /// Returns currently applied filter value.
    pub fn filter(&self) -> Option<&str> {
        self.filter.pattern()
    }

    /// Filters items in the list by calling `is_matching` on each [`Filterable`] row.\
    /// Returns `true` if pattern was updated.
    pub fn set_filter(&mut self, filter: Option<String>) -> bool {
        if !self.filter.set_pattern(filter) {
            return false;
        }

        if self.filter.has_pattern() {
            self.deselect_all();
            self.apply_filter();
        } else {
            self.items.filter_reset();
        }

        self.recover_highlighted_item_index();
        self.items.full_iter_mut().for_each(|i| i.is_active = false);
        if let Some(highlighted) = self.highlighted {
            self.items[highlighted].is_active = true;
        }

        true
    }

    /// Returns filter settings for the list.
    pub fn filter_settings(&self) -> Option<&str> {
        self.filter.settings()
    }

    /// Sets filter settings for the list.
    pub fn set_filter_settings(&mut self, settings: Option<impl Into<String>>) {
        self.filter.set_settings(settings);
        self.apply_filter();
    }

    /// Process [`KeyCode`] to move over the list.
    pub fn process_key_event(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Home => self.move_highlighted(i32::MIN),
            KeyCode::Up => self.move_highlighted(-1),
            KeyCode::PageUp => self.move_highlighted(-i32::from(self.page_height)),
            KeyCode::Down => self.move_highlighted(1),
            KeyCode::PageDown => self.move_highlighted(i32::from(self.page_height)),
            KeyCode::End => self.move_highlighted(i32::MAX),
            _ => return false,
        }

        true
    }

    /// Process mouse `ScrollUp` event.
    pub fn process_scroll_up(&mut self) {
        if self.page_start > 0 {
            self.move_highlighted(-1);
            self.page_start = self.page_start.saturating_sub(1);
        }
    }

    /// Process mouse `ScrollDown` event.
    pub fn process_scroll_down(&mut self) {
        if self.page_start < self.items.len().saturating_sub(usize::from(self.page_height)) {
            self.move_highlighted(1);
            self.page_start = self.page_start.saturating_add(1);
        }
    }

    /// Returns current page height.
    pub fn page_height(&self) -> u16 {
        self.page_height
    }

    /// Sets page start and height.
    pub fn set_page(&mut self, page_start: usize, page_height: u16) {
        let len = self.items.len();
        let max_start = len.saturating_sub(page_height.into());

        self.page_start = page_start.min(max_start);
        self.page_height = page_height;
    }

    /// Updates page start for the current page size and highlighted resource item.
    pub fn update_page(&mut self, new_height: u16) {
        self.page_height = new_height;
        let height = usize::from(self.page_height);
        let highlighted = self.highlighted.unwrap_or(0);

        if height == 0 {
            self.page_start = highlighted;
            return;
        }

        if self.page_start > highlighted {
            self.page_start = highlighted;
        } else if highlighted >= self.page_start + height {
            self.page_start = highlighted - height + 1;
        }

        let len = self.items.len();
        if len <= height {
            self.page_start = 0;
        } else if self.page_start + height > len {
            self.page_start = len - height;
        }
    }

    /// Returns list items iterator for the current page.
    pub fn get_page(&self) -> impl Iterator<Item = &Item<T, Fc>> {
        self.items.iter().skip(self.page_start).take(self.page_height.into())
    }

    /// Removes all fixed items from the list.
    pub fn remove_fixed(&mut self) {
        self.items.full_retain(|item| !item.is_fixed);
        self.apply_filter();
    }

    /// Selects all items.
    pub fn select_all(&mut self) {
        self.items.iter_mut().for_each(|item| item.select(true));
    }

    /// Clears items selection.
    pub fn deselect_all(&mut self) {
        self.items.iter_mut().for_each(|item| item.select(false));
    }

    /// Inverts selection of items in list.
    pub fn invert_selection(&mut self) {
        self.items.iter_mut().for_each(Item::invert_selection);
    }

    /// Selects / deselects currently highlighted item.
    pub fn select_highlighted_item(&mut self) {
        if let Some(highlighted) = self.highlighted
            && highlighted < self.items.len()
        {
            self.items[highlighted].invert_selection();
        }
    }

    /// Selects items by provided uids.
    pub fn select_uids(&mut self, uids: &[impl AsRef<str>]) {
        self.items
            .iter_mut()
            .for_each(|item| item.select(uids.iter().any(|u| u.as_ref() == item.data.uid())));
    }

    /// Returns selected item names grouped in [`HashMap`].
    pub fn get_selected_items(&self) -> HashMap<&str, Vec<&str>> {
        let mut result: HashMap<&str, Vec<&str>> = HashMap::new();
        for item in self.items.iter().filter(|i| i.is_selected) {
            result.entry(item.data.group()).or_default().push(item.data.name());
        }

        result
    }

    /// Returns selected item uids as [`Vec`].
    pub fn get_selected_uids(&self) -> Vec<&str> {
        self.items.iter().filter(|i| i.is_selected).map(|i| i.data.uid()).collect()
    }

    /// Returns `true` if anything is selected.
    pub fn is_anything_selected(&self) -> bool {
        self.items.iter().any(|i| i.is_selected)
    }

    /// Returns the names of items on the current page along with their active status.
    pub fn get_paged_names(&self, width: usize) -> Vec<(String, bool)> {
        self.get_paged_names_with_description(width, "")
    }

    /// Returns the names of items on the current page along with their active status.\
    /// **Note** that the highlighted (active) item may include an additional `description`.
    pub fn get_paged_names_with_description(&self, width: usize, description: &str) -> Vec<(String, bool)> {
        let mut result = Vec::with_capacity(self.page_height.into());
        for item in self.get_page() {
            if item.is_active && !description.is_empty() {
                result.push((item.data.get_name_with_description(width, description), true));
            } else {
                result.push((item.data.get_name(width), item.is_active));
            }
        }

        result
    }

    /// Gets highlighted element index.
    pub fn get_highlighted_item_index(&self) -> Option<usize> {
        self.highlighted
    }

    /// Gets highlighted element name.
    pub fn get_highlighted_item_name(&self) -> Option<&str> {
        self.get_highlighted_item().map(|i| i.data.name())
    }

    /// Gets highlighted element `uid`.
    pub fn get_highlighted_item_uid(&self) -> Option<&str> {
        self.get_highlighted_item().map(|i| i.data.uid())
    }

    /// Gets highlighted element.
    pub fn get_highlighted_item(&self) -> Option<&Item<T, Fc>> {
        let highlighted = self.highlighted?;
        (highlighted < self.items.len()).then(|| &self.items[highlighted])
    }

    /// Returns line number for the highlighted item for the current page.
    pub fn get_highlighted_item_line_no(&self) -> Option<u16> {
        let highlighted = self.highlighted?;
        if highlighted < self.items.len() && highlighted >= self.page_start {
            u16::try_from(highlighted - self.page_start).ok()
        } else {
            None
        }
    }

    /// Returns `true` if anything is highlighted.
    pub fn is_anything_highlighted(&self) -> bool {
        self.get_highlighted_item().is_some()
    }

    /// Recovers and returns the highlighted item index from the `is_active` property.
    pub fn recover_highlighted_item_index(&mut self) -> Option<usize> {
        self.highlighted = self.items.iter().position(|i| i.is_active);
        self.highlighted
    }

    /// Recovers remembered filter from the filter context.
    pub fn recover_filter(&mut self) {
        self.apply_filter();
        self.recover_highlighted_item_index();
    }

    /// Highlights element on list by its name.
    pub fn highlight_item_by_name(&mut self, name: &str) -> bool {
        self.highlight_item_by(|i| i.data.is_equal(name))
    }

    /// Highlights first element on the list which name starts with `text`.
    pub fn highlight_item_by_name_start(&mut self, text: &str) -> bool {
        self.highlight_item_by(|i| i.data.is_equal(text)) || self.highlight_item_by(|i| i.data.starts_with(text))
    }

    /// Highlights element on list by its `uid`.
    pub fn highlight_item_by_uid(&mut self, uid: &str) -> bool {
        self.highlight_item_by(|i| i.data.uid() == uid)
    }

    /// Highlights element on list by the visible line number.
    pub fn highlight_item_by_line(&mut self, line_no: u16) -> bool {
        let index = self.page_start + usize::from(line_no);
        if index >= self.items.len() {
            return false;
        }

        if let Some(highlighted) = self.highlighted
            && highlighted < self.items.len()
        {
            if highlighted == index {
                return true;
            }

            self.items[highlighted].is_active = false;
        }

        self.items[index].is_active = true;
        self.highlighted = Some(index);
        true
    }

    /// Tries to highlight item finding it by closure.
    pub fn highlight_item_by<F>(&mut self, f: F) -> bool
    where
        F: Fn(&Item<T, Fc>) -> bool,
    {
        let Some(index) = self.items.iter().position(f) else {
            return false;
        };

        if let Some(highlighted) = self.highlighted
            && highlighted < self.items.len()
        {
            self.items[highlighted].is_active = false;
        }

        self.items[index].is_active = true;
        self.highlighted = Some(index);
        true
    }

    /// Highlights first item on the list, returns `true` on success.
    pub fn highlight_first_item(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }

        if let Some(highlighted) = self.highlighted
            && highlighted < self.items.len()
        {
            self.items[highlighted].is_active = false;
        }

        self.items[0].is_active = true;
        self.highlighted = Some(0);
        true
    }

    /// Unhighlights any highlighted item.
    pub fn unhighlight_item(&mut self) {
        if let Some(highlighted) = self.highlighted
            && highlighted < self.items.len()
        {
            self.items[highlighted].is_active = false;
        }

        self.highlighted = None;
    }

    /// Adds `rows_to_move` to the currently highlighted item index.
    fn move_highlighted(&mut self, rows_to_move: i32) {
        if self.items.is_empty() || rows_to_move == 0 {
            return;
        }

        if self.highlighted.is_none() && rows_to_move > 0 {
            self.items[0].is_active = true;
            self.highlighted = Some(0);
            return;
        }

        let highlighted = self.highlighted.unwrap_or(0);
        if highlighted < self.items.len() {
            self.items[highlighted].is_active = false;
        }

        let last = self.items.len().saturating_sub(1);
        let new_highlighted = if rows_to_move > 0 {
            highlighted.saturating_add(rows_to_move as usize).min(last)
        } else {
            highlighted.saturating_sub(rows_to_move.unsigned_abs() as usize)
        };

        self.items[new_highlighted].is_active = true;
        self.highlighted = Some(new_highlighted);
    }

    /// Re-applies remembered text filter to the list.
    fn apply_filter(&mut self) {
        if let Some(context) = self.filter.context_mut() {
            context.restart();
            self.items.filter(context);
        } else if let Some(filter) = self.filter.pattern() {
            let mut context = T::get_context(filter, self.filter.settings());
            self.items.filter(&mut context);
            self.filter.set_context(Some(context));
        }
    }
}

/// Compares two [`Item`]s by selected column values ignoring fixed items.
fn cmp<T: Row + Filterable<Fc>, Fc: FilterContext>(a: &Item<T, Fc>, b: &Item<T, Fc>, column: usize) -> Ordering {
    if a.is_fixed || b.is_fixed {
        return Ordering::Equal;
    }

    a.data.column_sort_text(column).cmp(b.data.column_sort_text(column))
}
