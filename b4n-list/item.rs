use k8s_openapi::jiff::Timestamp;
use std::{borrow::Cow, marker::PhantomData};

use crate::{FilterContext, Filterable};

/// Contract for item with columns.
pub trait Row {
    /// Returns `uid` of the item.
    fn uid(&self) -> &str;

    /// Returns `group` of the item.
    fn group(&self) -> &str;

    /// Returns `name` of the item.
    fn name(&self) -> &str;

    /// Returns creation timestamp of the item.
    fn creation_timestamp(&self) -> Option<&Timestamp> {
        None
    }

    /// Returns `name` of the item respecting provided `width`.
    fn get_name(&self, width: usize) -> String;

    /// Returns the item's name with an added description, formatted to fit the given `width`.
    #[inline]
    fn get_name_with_description(&self, width: usize, _description: &str) -> String {
        self.get_name(width)
    }

    /// Returns text value for the specified column number.
    fn column_text(&self, column: usize) -> Cow<'_, str>;

    /// Returns text value for the specified column number that can be properly sorted.
    fn column_sort_text(&self, column: usize) -> &str;

    /// Returns `true` if the given `pattern` is found in the [`Row`] item.
    #[inline]
    fn contains(&self, pattern: &str) -> bool {
        self.name().contains(pattern)
    }

    /// Returns `true` if the [`Row`] item starts with the given `pattern`.
    #[inline]
    fn starts_with(&self, pattern: &str) -> bool {
        self.name().starts_with(pattern)
    }

    /// Returns `true` if the given `pattern` exactly matches the [`Row`] item.
    #[inline]
    fn is_equal(&self, pattern: &str) -> bool {
        self.name() == pattern
    }
}

/// Filterable list item.
pub struct Item<T: Row + Filterable<Fc>, Fc: FilterContext> {
    pub data: T,
    pub is_active: bool,
    pub is_selected: bool,
    pub is_dirty: bool,
    pub is_fixed: bool,
    _marker: PhantomData<Fc>,
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Item<T, Fc> {
    /// Creates new instance of a filterable list item.
    pub fn new(data: T) -> Self {
        Self {
            data,
            is_active: false,
            is_selected: false,
            is_dirty: false,
            is_fixed: false,
            _marker: PhantomData,
        }
    }

    /// Creates new dirty instance of a filterable list item.
    pub fn dirty(data: T) -> Self {
        let mut item = Item::new(data);
        item.is_dirty = true;
        item
    }

    /// Creates new fixed instance of a filterable list item.
    pub fn fixed(data: T) -> Self {
        let mut item = Item::new(data);
        item.is_fixed = true;
        item
    }

    /// Sets flag indicating if an item is selected.
    pub fn select(&mut self, is_selected: bool) {
        self.is_selected = !self.is_fixed && is_selected;
    }

    /// Inverts flag indicating if an item is selected.
    pub fn invert_selection(&mut self) {
        self.is_selected = !self.is_fixed && !self.is_selected;
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Filterable<Fc> for Item<T, Fc> {
    #[inline]
    fn get_context(pattern: &str, settings: Option<&str>) -> Fc {
        T::get_context(pattern, settings)
    }

    #[inline]
    fn is_matching(&self, context: &mut Fc) -> bool {
        self.data.is_matching(context)
    }
}
