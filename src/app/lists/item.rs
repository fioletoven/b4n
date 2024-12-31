/// Contract for item with columns
pub trait Row {
    /// Returns `group` of the item
    fn group(&self) -> &str;

    /// Returns `name` of the item
    fn name(&self) -> &str;

    /// Returns `name` of the item respecting provided `width`
    fn get_name(&self, width: usize) -> String;

    /// Returns text value for the specified column number
    fn column_text(&self, column: usize) -> &str;
}

/// List item
pub struct Item<T: Row> {
    pub data: T,
    pub is_active: bool,
    pub is_selected: bool,
    pub is_dirty: bool,
    pub is_fixed: bool,
}

impl<T: Row> Item<T> {
    /// Creates new instance of a list item
    pub fn new(data: T) -> Self {
        Self {
            data,
            is_active: false,
            is_selected: false,
            is_dirty: false,
            is_fixed: false,
        }
    }

    /// Creates new dirty instance of a list item
    pub fn dirty(data: T) -> Self {
        let mut item = Item::new(data);
        item.is_dirty = true;
        item
    }

    /// Creates new fixed instance of a list item
    pub fn fixed(data: T) -> Self {
        let mut item = Item::new(data);
        item.is_fixed = true;
        item
    }
}
