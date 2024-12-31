use std::collections::HashMap;

use colors::TextColors;
use crossterm::event::KeyEvent;
use theme::Theme;

pub use self::tui::*;

pub mod colors;
pub mod pages;
pub mod panes;
pub mod theme;
pub mod utils;
pub mod widgets;

mod tui;

/// Indicates which columns in the list should be displayed
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewType {
    /// Render rows with just the `name` column
    Name,

    /// Render rows without grouping column  
    /// _for k8s resource all columns except the `namespace` column_
    Compact,

    /// Render rows with all columns
    Full,
}

/// UI object that is responsive and can process key events
pub trait Responsive {
    /// Process UI key event
    fn process_key(&mut self, key: KeyEvent) -> ResponseEvent;
}

/// UI object that behaves like table
pub trait Table: Responsive {
    /// Returns the number of elements in the list.
    fn len(&self) -> usize;

    /// Returns `true` if list is filtered
    fn is_filtered(&self) -> bool;

    /// Filters list
    fn filter(&mut self, filter: Option<String>);

    /// Returns filter value
    fn get_filter(&self) -> Option<&str>;

    /// Sorts items in the list by column number
    fn sort(&mut self, column_no: usize, is_descending: bool);

    /// Gets highlighted element index
    fn get_highlighted_item_index(&self) -> Option<usize>;

    /// Gets highlighted element name
    fn get_highlighted_item_name(&self) -> Option<&str>;

    /// Highlights element on list by its name
    fn highlight_item_by_name(&mut self, name: &str);

    /// Highlights first element on list which name starts with `text`.  
    /// Returns `true` if element was found and selected.
    fn highlight_item_by_name_start(&mut self, text: &str);

    /// Highlights first item on list, returns `true` on success
    fn highlight_first_item(&mut self) -> bool;

    /// Clears selection of items
    fn deselect_all(&mut self);

    /// Inverts selection of items
    fn invert_selection(&mut self);

    /// Selects / deselects currently highlighted item
    fn select_highlighted_item(&mut self);

    /// Returns selected item names grouped in a [`HashMap`]
    fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;

    /// Returns `true` if any item in the list is selected
    fn is_anything_selected(&self) -> bool;

    /// Updates page start for the current page size and highlighted list item
    fn update_page(&mut self, new_height: u16);

    /// Returns item names from the current page and indications if item is active
    fn get_paged_names(&self, width: usize) -> Option<Vec<(String, bool)>>;

    /// Returns items from the current page in a form of text lines to display and colors for that lines
    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Option<Vec<(String, TextColors)>>;

    /// Returns header text for the list
    fn get_header(&self, view: ViewType, width: usize) -> String;
}
