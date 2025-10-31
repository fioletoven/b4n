use b4n_list::{FilterContext, Filterable, Row, ScrollableList};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::ui::{ResponseEvent, TuiEvent, lists::Header};

/// Extension trait for [`ScrollableList`].
pub trait ScrollableListExt {
    /// Processes a [`TuiEvent`] to move the highlight across list items.
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent;
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> ScrollableListExt for ScrollableList<T, Fc> {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        match event {
            TuiEvent::Key(key) => {
                if self.process_key_event(key.code) {
                    ResponseEvent::Handled
                } else {
                    ResponseEvent::NotHandled
                }
            },
            TuiEvent::Mouse(mouse) => {
                match mouse.kind {
                    crate::ui::MouseEventKind::ScrollDown => self.process_scroll_down(),
                    crate::ui::MouseEventKind::ScrollUp => self.process_scroll_up(),
                    _ => return ResponseEvent::NotHandled,
                }
                ResponseEvent::Handled
            },
        }
    }
}

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

    /// Process UI key or mouse event.
    pub fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        if let TuiEvent::Key(key) = event
            && key.modifiers == KeyModifiers::ALT
            && key.code != KeyCode::Char(' ')
            && let KeyCode::Char(code) = key.code
        {
            if code.is_numeric() {
                let sort_by = code.to_digit(10).unwrap() as usize;
                self.toggle_sort(sort_by);
                return ResponseEvent::Handled;
            }

            let sort_symbols = self.header.get_sort_symbols();
            let uppercase = code.to_ascii_uppercase();
            if let Some(sort_by) = sort_symbols.iter().position(|c| *c == uppercase) {
                self.toggle_sort(sort_by);
                return ResponseEvent::Handled;
            }
        }

        self.list.process_event(event)
    }

    /// Sorts the list.
    pub fn sort(&mut self, column_no: usize, is_descending: bool) {
        if column_no < self.header.get_columns_count() {
            self.header.set_sort_info(column_no, is_descending);
            self.sort_internal_list(column_no, is_descending);
        }
    }

    /// Toggles sorting for the specified column.\
    /// **Note** that if the column is already being used for sorting, the sort direction is reversed.
    pub fn toggle_sort(&mut self, column_no: usize) {
        let (old_column_no, is_descending) = self.header.sort_info();
        self.sort(column_no, if column_no == old_column_no { !is_descending } else { false });
    }

    /// Sorts the internal list.
    fn sort_internal_list(&mut self, column_no: usize, is_descending: bool) {
        let reverse = self.header.has_reversed_order(column_no);
        self.list
            .sort(column_no, if reverse { !is_descending } else { is_descending });
    }
}
