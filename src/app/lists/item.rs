use std::marker::PhantomData;

use super::{FilterContext, Filterable};

/// Contract for item with columns.
pub trait Row {
    /// Returns `uid` of the item.
    fn uid(&self) -> Option<&str>;

    /// Returns `group` of the item.
    fn group(&self) -> &str;

    /// Returns `name` of the item.
    fn name(&self) -> &str;

    /// Returns `name` of the item respecting provided `width`.
    fn get_name(&self, width: usize) -> String;

    /// Returns `name` for the highlighted item respecting provided `width`.
    #[inline]
    fn get_name_for_highlighted(&self, width: usize) -> String {
        self.get_name(width)
    }

    /// Returns text value for the specified column number.
    fn column_text(&self, column: usize) -> &str;

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
