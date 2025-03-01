use std::cmp::max;

use crate::utils::truncate;

#[cfg(test)]
#[path = "./column.tests.rs"]
mod column_tests;

/// Default `NAMESPACE` column.
pub const NAMESPACE: Column = Column {
    name: "NAMESPACE",
    is_fixed: false,
    to_right: false,
    is_sorted: false,
    min_len: 11,
    max_len: 11,
    data_len: 11,
};

/// Default `NAME` column.
pub const NAME: Column = Column {
    name: "NAME",
    is_fixed: false,
    to_right: false,
    is_sorted: true,
    min_len: 6,
    max_len: 6,
    data_len: 6,
};

/// Column for the list header.
#[derive(Clone)]
pub struct Column {
    pub name: &'static str,
    pub is_fixed: bool,
    pub to_right: bool,
    pub is_sorted: bool,
    pub data_len: usize,
    min_len: usize,
    max_len: usize,
}

impl Column {
    /// Creates new [`Column`] instance.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            is_fixed: false,
            to_right: false,
            is_sorted: false,
            data_len: name.chars().count(),
            min_len: name.chars().count(),
            max_len: name.chars().count(),
        }
    }

    /// Creates new [`Column`] instance bound with provided lengths.
    pub fn bound(name: &'static str, min_len: usize, max_len: usize, to_right: bool) -> Self {
        Self {
            name,
            is_fixed: false,
            to_right,
            is_sorted: false,
            data_len: name.chars().count(),
            min_len: max(name.chars().count(), min_len),
            max_len: max(name.chars().count(), max_len),
        }
    }

    /// Creates new fixed size [`Column`] instance.
    pub fn fixed(name: &'static str, len: usize, to_right: bool) -> Self {
        Self {
            name,
            is_fixed: true,
            to_right,
            is_sorted: false,
            data_len: len,
            min_len: max(name.chars().count(), len),
            max_len: max(name.chars().count(), len),
        }
    }

    /// Updates the value of `min_len` (and `max_len`, if necessary) to be valid for a first column.  
    /// **Note** that first column has one extra space in front of the header name.
    pub fn ensure_can_be_first_column(mut self) -> Self {
        if self.name.chars().count() == self.min_len {
            self.min_len += 1;
            if self.min_len > self.max_len {
                self.max_len = self.min_len
            }
        }

        self
    }

    #[inline]
    pub fn min_len(&self) -> usize {
        if self.is_sorted && self.name.chars().count() + 1 > self.min_len {
            self.min_len + 1
        } else {
            self.min_len
        }
    }

    #[inline]
    pub fn max_len(&self) -> usize {
        if self.is_sorted && self.min_len() > self.max_len {
            self.max_len + 1
        } else {
            self.max_len
        }
    }
}

/// Extension methods for string.
pub trait StringExtensions {
    /// Appends a given column onto the end of this `String`.
    fn push_column(&mut self, column: &Column, len: usize, is_descending: bool);
}

impl StringExtensions for String {
    fn push_column(&mut self, column: &Column, len: usize, is_descending: bool) {
        if len == 0 || (column.name.is_empty() && !column.is_sorted) {
            return;
        }

        let padding_len = len.saturating_sub(column.name.chars().count() + if column.is_sorted { 1 } else { 0 });
        if column.to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }

        for ch in truncate(column.name, len - if column.is_sorted { 1 } else { 0 }).chars() {
            self.push(if ch == ' ' { ' ' } else { ch })
        }

        if column.is_sorted {
            self.push(if is_descending { '↓' } else { '↑' });
        }

        if !column.to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }
    }
}
