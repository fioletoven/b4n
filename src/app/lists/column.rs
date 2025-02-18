use std::cmp::max;

/// Default `NAMESPACE` column
pub const NAMESPACE: Column = Column {
    name: "NAMESPACE",
    is_fixed: false,
    to_right: false,
    min_len: 11,
    max_len: 11,
    data_len: 11,
};

/// Default `NAME` column
pub const NAME: Column = Column {
    name: "NAME",
    is_fixed: false,
    to_right: false,
    min_len: 6,
    max_len: 6,
    data_len: 6,
};

/// Column for the list header
#[derive(Clone)]
pub struct Column {
    pub name: &'static str,
    pub is_fixed: bool,
    pub to_right: bool,
    pub min_len: usize,
    pub max_len: usize,
    pub data_len: usize,
}

impl Column {
    /// Creates new [`Column`] instance
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            is_fixed: false,
            to_right: false,
            min_len: name.len(),
            max_len: name.len(),
            data_len: name.len(),
        }
    }

    /// Creates new [`Column`] instance bound with provided lengths
    pub fn bound(name: &'static str, min_len: usize, max_len: usize, to_right: bool) -> Self {
        Self {
            name,
            is_fixed: false,
            to_right,
            min_len: max(name.len(), min_len),
            max_len: max(name.len(), max_len),
            data_len: name.len(),
        }
    }

    /// Creates new fixed size [`Column`] instance
    pub fn fixed(name: &'static str, len: usize, to_right: bool) -> Self {
        Self {
            name,
            is_fixed: true,
            to_right,
            min_len: max(name.len(), len),
            max_len: max(name.len(), len),
            data_len: len,
        }
    }

    /// Updates the value of `min_len` (and `max_len`, if necessary) to be valid for a first column.  
    /// **Note** that first column has one extra space in front of the header name.
    pub fn set_as_first_column(&mut self) {
        if self.name.len() == self.min_len {
            self.min_len += 1;
            if self.min_len > self.max_len {
                self.max_len = self.min_len
            }
        }
    }
}
