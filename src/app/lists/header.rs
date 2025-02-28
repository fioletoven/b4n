use std::rc::Rc;

use crate::{
    ui::ViewType,
    utils::{truncate, try_truncate},
};

use super::{Column, NAME};

#[cfg(test)]
#[path = "./header.tests.rs"]
mod header_tests;

/// Header for the list.
pub struct Header {
    group: Column,                        // column: 0, optional
    name: Column,                         // column: 1
    age: Column,                          // column: extra_columns len + 2 (last column)
    extra_columns: Option<Box<[Column]>>, // columns: 2 .. n
    extra_columns_text: String,
    all_extra_width: usize,
    extra_space: usize,
    sort_symbols: Rc<[char]>,
    sorted_column_no: usize,
    is_sorted_descending: bool,
}

impl Default for Header {
    fn default() -> Self {
        Self::from(Column::new("N/A"), None, Rc::new([' ', 'N', 'A']))
    }
}

impl Header {
    /// Creates new [`Header`] instance with provided columns.
    pub fn from(mut group_column: Column, extra_columns: Option<Box<[Column]>>, sort_symbols: Rc<[char]>) -> Self {
        let extra_columns_text = get_extra_columns_text(&extra_columns, false);
        let extra_width = extra_columns_text.chars().count() + 9; // AGE + all spaces = 9
        let extra_space = get_extra_space(&extra_columns);

        group_column.set_as_first_column();

        Self {
            group: group_column,
            name: NAME.clone(),
            age: Column::fixed("AGE", 6, true),
            extra_columns,
            extra_columns_text,
            all_extra_width: extra_width,
            extra_space,
            sort_symbols,
            sorted_column_no: 1,
            is_sorted_descending: false,
        }
    }

    /// Returns number of columns in the header.
    pub fn get_columns_count(&self) -> usize {
        if let Some(extra_columns) = &self.extra_columns {
            extra_columns.len() + 3
        } else {
            3
        }
    }

    /// Returns sorting symbols for columns.
    pub fn get_sort_symbols(&self) -> Rc<[char]> {
        Rc::clone(&self.sort_symbols)
    }

    /// Returns information required for sorting.
    pub fn sort_info(&self) -> (usize, bool) {
        (self.sorted_column_no, self.is_sorted_descending)
    }

    /// Sets information required for sorting.
    pub fn set_sort_info(&mut self, column_no: usize, is_descending: bool) {
        self.sorted_column_no = column_no;
        self.is_sorted_descending = is_descending;

        self.group.is_sorted = false;
        self.name.is_sorted = false;
        self.age.is_sorted = false;
        if let Some(columns) = &mut self.extra_columns {
            for column in columns.iter_mut() {
                column.is_sorted = false;
            }
        }

        if let Some(column) = self.column_mut(column_no) {
            column.is_sorted = true;
        }

        self.recalculate_extra_columns();
    }

    /// Recalculates extra columns text and width.
    pub fn recalculate_extra_columns(&mut self) {
        self.extra_columns_text = get_extra_columns_text(&self.extra_columns, self.is_sorted_descending);
        self.all_extra_width = self.extra_columns_text.chars().count() + 9; // AGE + all spaces = 9
        self.extra_space = get_extra_space(&self.extra_columns);
    }

    /// Resets `data_len` in each not fixed column.
    pub fn reset_data_lengths(&mut self) {
        self.group.data_len = 0;
        self.name.data_len = 0;
        if let Some(columns) = &mut self.extra_columns {
            for column in columns.iter_mut() {
                if !column.is_fixed {
                    column.data_len = 0;
                }
            }
        }
    }

    /// Returns current data length of the provided column.
    pub fn get_data_length(&self, column: usize) -> usize {
        self.column(column).map(|c| c.data_len).unwrap_or(3) // 3: "n/a" length
    }

    /// Sets data length for the provided column.
    pub fn set_data_length(&mut self, column: usize, new_data_len: usize) {
        if let Some(column) = self.column_mut(column) {
            if !column.is_fixed {
                column.data_len = new_data_len;
            }
        }
    }

    /// Returns extra columns.
    pub fn get_extra_columns(&self) -> Option<&[Column]> {
        self.extra_columns.as_deref()
    }

    /// Gets header text for the provided `group_width` and `name_width`.
    pub fn get_text(&self, view: ViewType, group_width: usize, name_width: usize, terminal_width: usize) -> String {
        let header = match view {
            ViewType::Name => self.get_name_text(name_width),
            ViewType::Compact => self.get_compact_text(name_width, terminal_width),
            ViewType::Full => self.get_full_text(group_width, name_width, terminal_width),
        };

        if terminal_width > 0 && header.chars().count() > terminal_width {
            if let Some(truncated) = try_truncate(header.as_str(), terminal_width) {
                return truncated.to_owned();
            }
        }

        header
    }

    /// Returns dynamic widths for name column together with extra space for it.
    pub fn get_widths(&self, terminal_width: usize) -> (usize, usize, usize) {
        if terminal_width <= self.name.min_len() + self.all_extra_width {
            (0, self.name.min_len(), self.extra_space)
        } else {
            (0, terminal_width - self.all_extra_width, self.extra_space)
        }
    }

    /// Returns dynamic widths for group and name columns together with extra space for name column.
    pub fn get_full_widths(&self, terminal_width: usize) -> (usize, usize, usize) {
        let min_width_for_all = self.group.min_len() + 1 + self.name.min_len() + self.all_extra_width;

        if terminal_width <= min_width_for_all {
            (self.group.min_len(), self.name.min_len(), self.extra_space)
        } else {
            let max_group_width = std::cmp::max(self.group.data_len, self.group.min_len());
            let min_width_for_full_size = max_group_width + 1 + self.name.data_len;

            if terminal_width >= min_width_for_full_size + self.all_extra_width {
                let avail_width = terminal_width - min_width_for_full_size - self.all_extra_width;

                (max_group_width, self.name.data_len + avail_width, self.extra_space)
            } else {
                let avail_width = terminal_width - min_width_for_all;
                let group_width = std::cmp::min(self.group.min_len() + avail_width / 2, max_group_width);
                let name_width = terminal_width - group_width - self.all_extra_width - 1;

                (group_width, name_width, self.extra_space)
            }
        }
    }

    /// Gets only name text.
    fn get_name_text(&self, name_width: usize) -> String {
        let mut header = String::with_capacity(name_width + 2);

        header.push(' ');
        header.push_column(&self.name, name_width.saturating_sub(1), self.is_sorted_descending);
        header.push(' ');

        header
    }

    /// Gets header text without group column.
    fn get_compact_text(&self, name_width: usize, terminal_width: usize) -> String {
        self.get_text_inner(0, name_width.saturating_sub(1), terminal_width, false)
    }

    /// Gets header text with group column.
    fn get_full_text(&self, group_width: usize, name_width: usize, terminal_width: usize) -> String {
        self.get_text_inner(group_width.saturating_sub(1), name_width, terminal_width, true)
    }

    fn get_text_inner(&self, group_width: usize, name_width: usize, terminal_width: usize, full: bool) -> String {
        let mut header = String::with_capacity(terminal_width + 2);

        if full {
            header.push(' ');
            header.push_column(&self.group, group_width, self.is_sorted_descending);
        }
        header.push(' ');
        header.push_column(&self.name, name_width, self.is_sorted_descending);
        header.push(' ');
        header.push_str(&self.extra_columns_text);
        header.push(' ');
        header.push_column(&self.age, self.age.max_len(), self.is_sorted_descending);
        header.push(' ');

        header
    }

    fn column(&self, column_no: usize) -> Option<&Column> {
        let Some(columns) = &self.extra_columns else {
            return match column_no {
                0 => Some(&self.group),
                1 => Some(&self.name),
                2 => Some(&self.age),
                _ => None,
            };
        };

        if column_no == 0 {
            Some(&self.group)
        } else if column_no == 1 {
            Some(&self.name)
        } else if column_no >= 2 && column_no <= columns.len() + 1 {
            Some(&columns[column_no - 2])
        } else if column_no == columns.len() + 2 {
            Some(&self.age)
        } else {
            None
        }
    }

    fn column_mut(&mut self, column_no: usize) -> Option<&mut Column> {
        let Some(columns) = &mut self.extra_columns else {
            return match column_no {
                0 => Some(&mut self.group),
                1 => Some(&mut self.name),
                2 => Some(&mut self.age),
                _ => None,
            };
        };

        if column_no == 0 {
            Some(&mut self.group)
        } else if column_no == 1 {
            Some(&mut self.name)
        } else if column_no >= 2 && column_no <= columns.len() + 1 {
            Some(&mut columns[column_no - 2])
        } else if column_no == columns.len() + 2 {
            Some(&mut self.age)
        } else {
            None
        }
    }
}

/// Builds extra columns text.
fn get_extra_columns_text(extra_columns: &Option<Box<[Column]>>, is_descending: bool) -> String {
    let Some(columns) = &extra_columns else {
        return String::new();
    };

    let header_len = columns.iter().map(|c| c.max_len() + 2).sum::<usize>() + 1;
    let mut header_text = String::with_capacity(header_len);
    for (i, column) in columns.iter().enumerate() {
        if i > 0 {
            header_text.push(' ');
        }

        header_text.push_column(
            column,
            column.data_len.clamp(column.min_len(), column.max_len()),
            is_descending,
        );
    }

    header_text
}

/// Returns extra space (if available) from the first additional column:
/// ```ignore
/// NAME  RESTARTS  
/// XXXXXXXXXX YYY  
///       ^^^^^
/// ```
/// In this case extra space is equal 5 as `restarts` column has 5 spare spaces before data starts.
fn get_extra_space(extra_columns: &Option<Box<[Column]>>) -> usize {
    let Some(columns) = &extra_columns else {
        return 0;
    };

    if columns.len() > 0 && columns[0].to_right && columns[0].min_len() > columns[0].data_len {
        columns[0].min_len() - columns[0].data_len
    } else {
        0
    }
}

/// Extension methods for string.
trait StringExtensions {
    fn push_column(&mut self, column: &Column, len: usize, is_descending: bool);
}

impl StringExtensions for String {
    /// Appends a given column onto the end of this `String`.
    fn push_column(&mut self, column: &Column, len: usize, is_descending: bool) {
        if len == 0 || column.name.is_empty() {
            return;
        }

        let padding_len = len.saturating_sub(column.name.chars().count() + if column.is_sorted { 1 } else { 0 });
        if column.to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }

        self.push_str(truncate(column.name, len - if column.is_sorted { 1 } else { 0 }));
        if column.is_sorted {
            self.push(if is_descending { '↓' } else { '↑' });
        }

        if !column.to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }
    }
}
