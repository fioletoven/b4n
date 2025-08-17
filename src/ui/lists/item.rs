use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use std::{borrow::Cow, marker::PhantomData};

use crate::{
    ui::{
        ViewType,
        lists::{AGE_COLUMN_WIDTH, Header},
    },
    utils::truncate,
};

use super::{FilterContext, Filterable};

#[cfg(test)]
#[path = "./item.tests.rs"]
mod item_tests;

/// Contract for item with columns.
pub trait Row {
    /// Returns `uid` of the item.
    fn uid(&self) -> Option<&str>;

    /// Returns `group` of the item.
    fn group(&self) -> &str;

    /// Returns `name` of the item.
    fn name(&self) -> &str;

    /// Returns creation timestamp of the item.
    fn creation_timestamp(&self) -> Option<&Time> {
        None
    }

    /// Returns `name` of the item respecting provided `width`.
    fn get_name(&self, width: usize) -> String;

    /// Returns `name` for the highlighted item respecting provided `width`.
    #[inline]
    fn get_name_for_highlighted(&self, width: usize) -> String {
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

    /// Builds and returns the whole row of values for this item.
    pub fn get_text(&self, view: ViewType, header: &Header, width: usize, namespace_width: usize, name_width: usize) -> String {
        let mut row = String::with_capacity(width + 2);
        match view {
            ViewType::Name => row.push_cell(self.data.name(), width, false),
            ViewType::Compact => self.get_compact_text(&mut row, header, name_width),
            ViewType::Full => self.get_full_text(&mut row, header, namespace_width, name_width),
        }

        if row.chars().count() > width {
            truncate(row.as_str(), width).to_owned()
        } else {
            row
        }
    }

    fn get_compact_text(&self, row: &mut String, header: &Header, name_width: usize) {
        row.push_cell(self.data.name(), name_width, false);
        row.push(' ');
        self.push_inner_text(row, header);
        row.push(' ');
        row.push_cell(
            self.data
                .creation_timestamp()
                .map(crate::kubernetes::utils::format_timestamp)
                .as_deref()
                .unwrap_or("n/a"),
            AGE_COLUMN_WIDTH + 1,
            true,
        );
    }

    fn get_full_text(&self, row: &mut String, header: &Header, namespace_width: usize, name_width: usize) {
        row.push_cell(self.data.column_text(0).as_ref(), namespace_width, false);
        row.push(' ');
        self.get_compact_text(row, header, name_width);
    }

    fn push_inner_text(&self, row: &mut String, header: &Header) {
        let Some(columns) = header.get_extra_columns() else {
            return;
        };

        for (i, _) in columns.iter().enumerate() {
            if i > 0 {
                row.push(' ');
            }

            let len = if i == 0 && columns[i].to_right {
                columns[i].data_len
            } else {
                columns[i].len()
            };

            row.push_cell(self.data.column_text(i + 2).as_ref(), len, columns[i].to_right);
        }
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

/// Extension methods for string.
pub trait RowStringExt {
    /// Appends a given cell text onto the end of this `String`.
    fn push_cell(&mut self, s: &str, len: usize, to_right: bool);
}

impl RowStringExt for String {
    fn push_cell(&mut self, s: &str, len: usize, to_right: bool) {
        if len == 0 {
            return;
        }

        let padding_len = len.saturating_sub(s.chars().count());
        if to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }

        self.push_str(truncate(s, len));

        if !to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }
    }
}
