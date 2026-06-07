use b4n_config::themes::{TextColors, Theme};
use b4n_list::Item;
use b4n_list::Row;
use b4n_tui::table::{Column, Header, ItemExt, NONE, TabularList, ViewType};
use b4n_tui::{ResponseEvent, Responsive, TuiEvent, table::Table};
use delegate::delegate;
use std::{collections::HashMap, rc::Rc};

use crate::ui::widgets::table::BasicRow;

/// Basic table.
pub struct BasicTable {
    pub table: TabularList<BasicRow, b4n_list::BasicFilterContext>,
    is_focused: bool,
}

impl Default for BasicTable {
    fn default() -> Self {
        Self::new(Column::new("NAME"), Box::default(), &['N'])
    }
}

impl BasicTable {
    /// Creates a new [`BasicTable`] with custom columns.
    pub fn new(name_column: Column, extra_columns: Box<[Column]>, sort_symbols: &[char]) -> Self {
        let mut all_symbols = vec![' ']; // namespace column, hidden
        all_symbols.extend_from_slice(sort_symbols); // name column + extra columns, visible
        all_symbols.push(' '); // age column, hidden

        let header = Header::from(NONE, Some(extra_columns), all_symbols.into())
            .with_name_column(name_column)
            .with_age_column(false)
            .with_stretch_last();

        Self {
            table: TabularList::new(header),
            is_focused: true,
        }
    }

    /// Sets `is_focused` for basic table.
    pub fn with_focus(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Sets first column to be stretched instead of the default one (last).
    pub fn with_stretch_name(mut self) -> Self {
        self.table.header.set_stretch_last(false);
        self
    }

    /// Inserts or removes a single row while preserving current sort order.
    pub fn update(&mut self, row: BasicRow, is_delete: bool) {
        if is_delete {
            let index = self.table.list.full_iter().position(|r| r.data.uid() == row.uid());
            if let Some(index) = index {
                self.table.list.full_remove(index);
            }
        } else if let Some(existing) = self.table.list.full_iter_mut().find(|r| r.data.uid() == row.uid()) {
            existing.data = row;
            existing.is_dirty = true;
        } else {
            self.table.list.push(Item::dirty(row));
        }

        let (sort_by, is_descending) = self.table.header.sort_info();
        self.table.sort(sort_by, is_descending);
        self.table.update_data_lengths();
    }

    /// Removes row by uid.
    pub fn remove(&mut self, row_uid: &str) {
        let index = self.table.list.full_iter().position(|r| r.data.uid() == row_uid);
        if let Some(index) = index {
            self.table.list.full_remove(index);
            self.table.update_data_lengths();
        }
    }

    /// Returns rows as formatted strings.
    pub fn get_items_as_text(&mut self, view: ViewType, selected: bool) -> Vec<String> {
        self.table.get_items_as_text(view, selected)
    }
}

impl Responsive for BasicTable {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        self.table.process_event(event)
    }
}

impl Table for BasicTable {
    delegate! {
        to self.table.list {
            fn len(&self) -> usize;
            fn is_filtered(&self) -> bool;
            fn set_filter(&mut self, filter: Option<String>);
            fn filter(&self) -> Option<&str>;
            fn is_anything_highlighted(&self) -> bool;
            fn get_highlighted_item_index(&self) -> Option<usize>;
            fn get_highlighted_item_name(&self) -> Option<&str>;
            fn get_highlighted_item_uid(&self) -> Option<&str>;
            fn get_highlighted_item_line_no(&self) -> Option<u16>;
            fn highlight_item_by_name(&mut self, name: &str) -> bool;
            fn highlight_item_by_name_start(&mut self, text: &str) -> bool;
            fn highlight_item_by_uid(&mut self, uid: &str) -> bool;
            fn highlight_item_by_line(&mut self, line_no: u16) -> bool;
            fn highlight_first_item(&mut self) -> bool;
            fn unhighlight_item(&mut self);
            fn select_all(&mut self);
            fn deselect_all(&mut self);
            fn invert_selection(&mut self);
            fn select_highlighted_item(&mut self);
            fn get_selected_items(&self) -> HashMap<&str, Vec<&str>>;
            fn is_anything_selected(&self) -> bool;
            fn set_page(&mut self, page_start: usize, page_height: u16);
            fn update_page(&mut self, new_height: u16);
            fn get_paged_names(&self, width: usize) -> Vec<(String, bool)>;
        }
    }

    fn clear(&mut self) {
        let (sort_by, is_descending) = self.table.header.sort_info();
        self.table.list.clear();
        self.table.sort(sort_by, is_descending);
        self.table.update_data_lengths();
    }

    fn set_focus(&mut self, is_focused: bool) {
        self.is_focused = is_focused;
    }

    fn get_column_at_position(&self, position: usize) -> Option<usize> {
        self.table.get_column_at_position(position)
    }

    fn sort(&mut self, column_no: usize, is_descending: bool) {
        self.table.sort(column_no, is_descending);
    }

    fn toggle_sort(&mut self, column_no: usize) {
        self.table.toggle_sort(column_no);
    }

    fn get_sort_symbols(&self) -> Rc<[char]> {
        self.table.header.get_sort_symbols()
    }

    fn get_paged_items(&self, theme: &Theme, view: ViewType, width: usize) -> Vec<(String, TextColors)> {
        let widths = self.table.header.get_widths(view, width);

        let mut result = Vec::with_capacity(self.table.list.page_height().into());
        for item in self.table.list.get_page() {
            let colors = if self.is_focused {
                theme.colors.list.line.ready.get_specific(item.is_active, item.is_selected)
            } else {
                theme.colors.list.line.dimmed.get_specific(item.is_active, item.is_selected)
            };
            result.push((
                item.get_text(view, &self.table.header, &widths, width, self.table.offset()),
                colors,
            ));
        }

        result
    }

    fn get_header(&mut self, view: ViewType, width: usize) -> &str {
        self.table.header.get_text(view, width)
    }

    fn refresh_header(&mut self, view: ViewType, width: usize) {
        self.table.header.refresh_text(view, width);
    }

    fn offset(&self) -> usize {
        self.table.offset()
    }

    fn refresh_offset(&mut self) -> usize {
        self.table.get_offset()
    }
}
