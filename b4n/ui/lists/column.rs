use std::{borrow::Cow, cmp::max};

use b4n_utils::truncate;

use crate::kubernetes::resources::CrdColumn;

#[cfg(test)]
#[path = "./column.tests.rs"]
mod column_tests;

pub const AGE_COLUMN_WIDTH: usize = 6;

/// Default `NAMESPACE` column.
pub const NAMESPACE: Column = Column {
    name: Cow::Borrowed("NAMESPACE"),
    is_fixed: false,
    to_right: false,
    is_sorted: false,
    has_reversed_order: false,
    min_len: 11,
    max_len: 11,
    data_len: 11,
};

/// Default `NAME` column.
pub const NAME: Column = Column {
    name: Cow::Borrowed("NAME"),
    is_fixed: false,
    to_right: false,
    is_sorted: true,
    has_reversed_order: false,
    min_len: 6,
    max_len: 6,
    data_len: 6,
};

/// Default `AGE` column.
pub const AGE: Column = Column {
    name: Cow::Borrowed("AGE"),
    is_fixed: true,
    to_right: true,
    is_sorted: false,
    has_reversed_order: true,
    min_len: AGE_COLUMN_WIDTH,
    max_len: AGE_COLUMN_WIDTH,
    data_len: AGE_COLUMN_WIDTH,
};

/// Column for the list header.
#[derive(Clone)]
pub struct Column {
    pub name: Cow<'static, str>,
    pub is_fixed: bool,
    pub to_right: bool,
    pub is_sorted: bool,
    pub has_reversed_order: bool,
    pub data_len: usize,
    min_len: usize,
    max_len: usize,
}

impl Column {
    /// Creates new [`Column`] instance.
    pub fn new(name: &'static str) -> Self {
        let len = name.chars().count();
        Self {
            name: Cow::Borrowed(name),
            is_fixed: false,
            to_right: false,
            is_sorted: false,
            has_reversed_order: false,
            data_len: len,
            min_len: len,
            max_len: len,
        }
    }

    /// Creates new [`Column`] instance bound with provided lengths.
    pub fn bound(name: &'static str, min_len: usize, max_len: usize, to_right: bool) -> Self {
        let len = name.chars().count();
        Self {
            name: Cow::Borrowed(name),
            is_fixed: false,
            to_right,
            is_sorted: false,
            has_reversed_order: false,
            data_len: len,
            min_len: max(len, min_len),
            max_len: max(len, max_len),
        }
    }

    /// Creates new fixed size [`Column`] instance.
    pub fn fixed(name: &'static str, len: usize, to_right: bool) -> Self {
        let constrained_len = max(name.chars().count(), len);
        Self {
            name: Cow::Borrowed(name),
            is_fixed: true,
            to_right,
            is_sorted: false,
            has_reversed_order: false,
            data_len: len,
            min_len: constrained_len,
            max_len: constrained_len,
        }
    }

    /// Creates new [`Column`] instance from custom resource column definition.
    pub fn from(column: &CrdColumn) -> Self {
        let min_len = column.name.chars().count().clamp(10, 20);

        Self {
            name: Cow::Owned(column.name.to_uppercase()),
            is_fixed: false,
            to_right: column.field_type != "string",
            is_sorted: false,
            has_reversed_order: column.field_type == "date",
            data_len: 10,
            min_len,
            max_len: 50,
        }
    }

    /// Sets reversed order for the column.
    pub fn with_reversed_order(mut self) -> Self {
        self.has_reversed_order = true;
        self
    }

    /// Updates the value of `min_len` (and `max_len`, if necessary) to be valid for a first column.\
    /// **Note** that first column has one extra space in front of the header name.
    pub fn ensure_can_be_first_column(mut self) -> Self {
        if self.name.chars().count() == self.min_len {
            self.min_len += 1;
            if self.min_len > self.max_len {
                self.max_len = self.min_len;
            }
        }

        self
    }

    /// Returns the current length of a [`Column`].
    #[inline]
    pub fn len(&self) -> usize {
        self.data_len.clamp(self.min_len(), self.max_len())
    }

    /// Returns `true` if [`Column`] has a current length of zero bytes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `min` length of a [`Column`].
    #[inline]
    pub fn min_len(&self) -> usize {
        if self.is_sorted && self.name.chars().count() + 1 > self.min_len {
            self.min_len + 1
        } else {
            self.min_len
        }
    }

    /// Returns `max` length of a [`Column`].
    #[inline]
    pub fn max_len(&self) -> usize {
        if self.is_sorted && self.min_len() > self.max_len {
            self.max_len + 1
        } else {
            self.max_len
        }
    }
}

/// Column extension methods for string.
pub trait ColumnStringExt {
    /// Appends a given column onto the end of this `String`.
    fn push_column(&mut self, column: &Column, len: usize, is_descending: bool);
}

impl ColumnStringExt for String {
    fn push_column(&mut self, column: &Column, len: usize, is_descending: bool) {
        if len == 0 || (column.name.is_empty() && !column.is_sorted) {
            return;
        }

        let padding_len = len.saturating_sub(column.name.chars().count() + usize::from(column.is_sorted));
        if column.to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }

        for ch in truncate(column.name.as_ref(), len - usize::from(column.is_sorted)).chars() {
            self.push(if ch == ' ' { ' ' } else { ch });
        }

        if column.is_sorted {
            self.push(if is_descending { '↓' } else { '↑' });
        }

        if !column.to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }
    }
}
