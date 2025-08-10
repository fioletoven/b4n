use crossterm::event::{KeyCode, KeyModifiers};

use crate::ui::{
    ResponseEvent,
    lists::{FilterContext, Filterable, Header, Row, ScrollableList},
};

/// Tabular UI list.
pub struct TabularList<T: Row + Filterable<Fc>, Fc: FilterContext> {
    pub list: ScrollableList<T, Fc>,
    pub header: Header,
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Default for TabularList<T, Fc> {
    fn default() -> Self {
        Self {
            list: ScrollableList::default(),
            header: Header::default(),
        }
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> TabularList<T, Fc> {
    /// Updates max widths for all columns basing on current data in the list.
    pub fn update_data_lengths(&mut self) {
        self.header.reset_data_lengths();

        let Some(list) = &self.list.items else {
            return;
        };

        let columns_no = self.header.get_columns_count();
        for item in list {
            for column in 0..columns_no {
                let column_width = std::cmp::max(
                    self.header.get_data_length(column),
                    item.data.column_text(column).chars().count(),
                );
                self.header.set_data_length(column, column_width);
            }
        }

        self.header.recalculate_extra_columns();
    }

    pub fn process_key(&mut self, key: crossterm::event::KeyEvent) -> ResponseEvent {
        if key.modifiers == KeyModifiers::ALT
            && key.code != KeyCode::Char(' ')
            && let KeyCode::Char(code) = key.code
        {
            if code.is_numeric() {
                let (column_no, is_descending) = self.header.sort_info();
                let sort_by = code.to_digit(10).unwrap() as usize;
                self.sort(sort_by, if sort_by == column_no { !is_descending } else { false });
                return ResponseEvent::Handled;
            }

            let sort_symbols = self.header.get_sort_symbols();
            let uppercase = code.to_ascii_uppercase();
            let sort_by = sort_symbols.iter().position(|c| *c == uppercase);
            if let Some(sort_by) = sort_by {
                let (column_no, is_descending) = self.header.sort_info();
                self.sort(sort_by, if sort_by == column_no { !is_descending } else { false });
                return ResponseEvent::Handled;
            }
        }

        self.list.process_key(key)
    }

    pub fn sort(&mut self, column_no: usize, is_descending: bool) {
        if column_no < self.header.get_columns_count() {
            self.header.set_sort_info(column_no, is_descending);
            self.sort_internal_list(column_no, is_descending);
        }
    }

    /// Sorts internal items list.
    fn sort_internal_list(&mut self, column_no: usize, is_descending: bool) {
        let reverse = self.header.has_reversed_order(column_no);
        self.list
            .sort(column_no, if reverse { !is_descending } else { is_descending });
    }
}
