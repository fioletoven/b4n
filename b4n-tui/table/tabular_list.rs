use b4n_config::keys::KeyCombination;
use b4n_list::{FilterContext, Filterable, Row, ScrollableList};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::table::Header;
use crate::{MouseEvent, MouseEventKind, ResponseEvent, Responsive, TuiEvent};

/// Indicates which columns in the list should be displayed.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum ViewType {
    /// Render rows with just the `name` column
    Name,

    /// Render rows without grouping column
    /// _for k8s resource all columns except the `namespace` column_
    Compact,

    /// Render rows with all columns
    #[default]
    Full,
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Responsive for ScrollableList<T, Fc> {
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
                    MouseEventKind::ScrollDown => self.process_scroll_down(),
                    MouseEventKind::ScrollUp => self.process_scroll_up(),
                    _ => return ResponseEvent::NotHandled,
                }
                ResponseEvent::Handled
            },
        }
    }
}

/// Tabular UI list.
pub struct TabularList<T: Row + Filterable<Fc>, Fc: FilterContext> {
    pub header: Header,
    pub list: ScrollableList<T, Fc>,
    width: usize,
    length: usize,
    offset: usize,
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Default for TabularList<T, Fc> {
    fn default() -> Self {
        Self {
            header: Header::default(),
            list: ScrollableList::default(),
            width: 0,
            length: 0,
            offset: 0,
        }
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> Responsive for TabularList<T, Fc> {
    fn process_event(&mut self, event: &TuiEvent) -> ResponseEvent {
        match event {
            TuiEvent::Key(key) => {
                if self.process_key_event(key) == ResponseEvent::Handled {
                    return ResponseEvent::Handled;
                }
            },
            TuiEvent::Mouse(mouse) => {
                if self.process_mouse_event(*mouse) == ResponseEvent::Handled {
                    return ResponseEvent::Handled;
                }
            },
        }

        self.list.process_event(event)
    }
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> TabularList<T, Fc> {
    /// Creates new [`TabularList`] instance.
    pub fn new(header: Header) -> Self {
        Self {
            header,
            ..Default::default()
        }
    }

    /// Sets new header for the table.\
    /// **Note** that it also clears the list items.
    pub fn update_header(&mut self, new_header: Header) {
        self.header = new_header;
        self.list.clear();
        self.offset = 0;
    }

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

    /// Returns column number located at the specified character position.
    pub fn get_column_at_position(&self, position: usize) -> Option<usize> {
        if position < self.header.get_cached_length().unwrap_or_default() {
            Some(count_columns_up_to(self.header.get_cached_text(), position))
        } else {
            None
        }
    }

    /// Sorts the list.
    pub fn sort(&mut self, column_no: usize, is_descending: bool) {
        if column_no < self.header.get_columns_count() {
            let view = self.header.get_cached_view();
            let width = self.header.get_cached_width();
            self.header.set_sort_info(column_no, is_descending);
            self.sort_internal_list(column_no, is_descending);
            if let Some(width) = width {
                self.header.refresh_text(view, width);
            }
            self.recalculate_offset();
        }
    }

    /// Toggles sorting for the specified column.\
    /// **Note** that if the column is already being used for sorting, the sort direction is reversed.
    pub fn toggle_sort(&mut self, column_no: usize) {
        let (old_column_no, is_descending) = self.header.sort_info();
        self.sort(column_no, if column_no == old_column_no { !is_descending } else { false });
    }

    /// Sets the current horizontal offset of the table.
    pub fn set_offset(&mut self, offset: usize) {
        self.width = self.header.get_cached_width().unwrap_or_default();
        self.length = self.header.get_cached_length().unwrap_or_default();
        self.offset = offset.min(self.length.saturating_sub(self.width));
    }

    /// Gets the current horizontal offset of the table recalculating it if needed.
    pub fn get_offset(&mut self) -> usize {
        self.recalculate_offset();
        self.offset
    }

    /// Gets the current horizontal offset of the table.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Sorts the internal list.
    fn sort_internal_list(&mut self, column_no: usize, is_descending: bool) {
        let reverse = self.header.has_reversed_order(column_no);
        self.list
            .sort(column_no, if reverse { !is_descending } else { is_descending });
    }

    fn recalculate_offset(&mut self) {
        if let Some(width) = self.header.get_cached_width()
            && let Some(length) = self.header.get_cached_length()
            && (self.width != width || self.length != length)
        {
            self.width = width;
            self.length = length;
            self.offset = self.offset.min(self.length.saturating_sub(self.width));
        }
    }

    fn process_key_event(&mut self, key: &KeyCombination) -> ResponseEvent {
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Home => {
                    self.set_offset(0);
                    return ResponseEvent::Handled;
                },
                KeyCode::PageUp => {
                    let width = self.header.get_cached_width().unwrap_or_default().saturating_div(2);
                    self.set_offset(self.offset.saturating_sub(width));
                    return ResponseEvent::Handled;
                },
                KeyCode::Left => {
                    self.set_offset(self.offset.saturating_sub(1));
                    return ResponseEvent::Handled;
                },
                KeyCode::Right => {
                    self.set_offset(self.offset + 1);
                    return ResponseEvent::Handled;
                },
                KeyCode::PageDown => {
                    let width = self.header.get_cached_width().unwrap_or_default().saturating_div(2);
                    self.set_offset(self.offset + width);
                    return ResponseEvent::Handled;
                },
                KeyCode::End => {
                    self.set_offset(self.header.get_cached_length().unwrap_or_default());
                    return ResponseEvent::Handled;
                },
                _ => (),
            }
        }

        if key.modifiers == KeyModifiers::ALT
            && key.code != KeyCode::Char(' ')
            && let KeyCode::Char(code) = key.code
        {
            if let Some(sort_by) = code.to_digit(10) {
                self.toggle_sort(sort_by as usize);
                return ResponseEvent::Handled;
            }

            let sort_symbols = self.header.get_sort_symbols();
            let uppercase = code.to_ascii_uppercase();
            if let Some(sort_by) = sort_symbols.iter().position(|c| *c == uppercase) {
                self.toggle_sort(sort_by);
                return ResponseEvent::Handled;
            }
        }

        ResponseEvent::NotHandled
    }

    fn process_mouse_event(&mut self, mouse: MouseEvent) -> ResponseEvent {
        if mouse.kind == MouseEventKind::ScrollLeft
            || (mouse.kind == MouseEventKind::ScrollUp && mouse.modifiers == KeyModifiers::CONTROL)
        {
            self.set_offset(self.offset.saturating_sub(1));
            return ResponseEvent::Handled;
        } else if mouse.kind == MouseEventKind::ScrollRight
            || (mouse.kind == MouseEventKind::ScrollDown && mouse.modifiers == KeyModifiers::CONTROL)
        {
            self.set_offset(self.offset + 1);
            return ResponseEvent::Handled;
        }

        ResponseEvent::NotHandled
    }
}

fn count_columns_up_to(text: &str, position: usize) -> usize {
    let mut in_column = false;
    let mut column_count = 0;

    for (i, c) in text.chars().enumerate() {
        if i > position {
            break;
        }

        if c == ' ' {
            in_column = false;
        } else if !in_column {
            column_count += 1;
            in_column = true;
        }
    }

    column_count
}
