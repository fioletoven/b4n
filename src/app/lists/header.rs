use crate::{
    ui::ViewType,
    utils::{add_padding, try_truncate},
};

use super::{Column, NAME};

/// Header for the list
pub struct Header {
    group: Column,                        // column: 0, optional
    name: Column,                         // column: 1
    age: Column,                          // column: extra_columns.len() + 2 (last column)
    extra_columns: Option<Box<[Column]>>, // columns: 2 .. n
    extra_columns_text: String,
    all_extra_width: usize,
    extra_space: usize,
}

impl Header {
    /// Creates new [`Header`] instance
    pub fn new() -> Self {
        Self::from(Column::new("N/A"), None)
    }

    /// Creates new [`Header`] instance with provided columns
    pub fn from(group_column: Column, extra_columns: Option<Box<[Column]>>) -> Self {
        let extra_columns_text = get_extra_columns_text(&extra_columns);
        let extra_width = extra_columns_text.len() + 9; // AGE + all spaces = 9
        let extra_space = get_extra_space(&extra_columns);

        Self {
            group: group_column,
            name: NAME.clone(),
            age: Column::fixed("AGE", 6, true),
            extra_columns,
            extra_columns_text,
            all_extra_width: extra_width,
            extra_space,
        }
    }

    /// Returns number of columns in the header
    pub fn get_columns_count(&self) -> usize {
        if let Some(extra_columns) = &self.extra_columns {
            extra_columns.len() + 3
        } else {
            3
        }
    }

    /// Recalculates extra columns text and width
    pub fn recalculate_extra_columns(&mut self) {
        self.extra_columns_text = get_extra_columns_text(&self.extra_columns);
        self.all_extra_width = self.extra_columns_text.len() + 9; // AGE + all spaces = 9
        self.extra_space = get_extra_space(&self.extra_columns);
    }

    /// Resets `data_len` in each not fixed column
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

    /// Returns current data length of the provided column
    pub fn get_data_length(&self, column: usize) -> usize {
        let Some(columns) = &self.extra_columns else {
            return match column {
                0 => self.group.data_len,
                1 => self.name.data_len,
                2 => self.age.data_len,
                _ => 3, // "n/a" length
            };
        };

        if column == 0 {
            self.group.data_len
        } else if column == 1 {
            self.name.data_len
        } else if column >= 2 && column <= columns.len() + 1 {
            columns[column - 2].data_len
        } else if column == columns.len() + 2 {
            self.age.data_len
        } else {
            3 // "n/a" length
        }
    }

    /// Sets data length for the provided column
    pub fn set_data_length(&mut self, column: usize, new_data_len: usize) {
        let Some(columns) = &mut self.extra_columns else {
            match column {
                0 => self.group.data_len = new_data_len,
                1 => self.name.data_len = new_data_len,
                _ => (),
            };
            return;
        };

        if column == 0 {
            self.group.data_len = new_data_len;
        } else if column == 1 {
            self.name.data_len = new_data_len;
        } else if column >= 2 && column <= columns.len() + 1 {
            if !columns[column - 2].is_fixed {
                columns[column - 2].data_len = new_data_len;
            }
        }
    }

    /// Returns extra columns
    pub fn get_extra_columns(&self) -> Option<&[Column]> {
        self.extra_columns.as_deref()
    }

    /// Gets header text for the provided `group_width` and `name_width`.
    /// If `force_width` is 0 it may exceed the desired width.
    pub fn get_text(&self, view: ViewType, group_width: usize, name_width: usize, force_width: usize) -> String {
        let header = match view {
            ViewType::Name => format!(" {} ", self.name.name),
            ViewType::Compact => self.get_compact_text(name_width),
            ViewType::Full => self.get_full_text(group_width, name_width),
        };

        if force_width > 0 {
            if let Some(truncated) = try_truncate(header.as_str(), force_width) {
                return truncated.to_owned();
            }
        }

        header
    }

    /// Returns dynamic widths for name column together with extra space for it
    pub fn get_widths(&self, terminal_width: usize) -> (usize, usize, usize) {
        if terminal_width <= self.name.min_len + self.all_extra_width {
            (0, self.name.min_len, self.extra_space)
        } else {
            (0, terminal_width - self.all_extra_width, self.extra_space)
        }
    }

    /// Returns dynamic widths for group and name columns together with extra space for name column
    pub fn get_full_widths(&self, terminal_width: usize) -> (usize, usize, usize) {
        let min_width_for_all = self.group.min_len + 1 + self.name.min_len + self.all_extra_width;

        if terminal_width <= min_width_for_all {
            (self.group.min_len, self.name.min_len, self.extra_space)
        } else {
            let max_group_width = std::cmp::max(self.group.data_len, self.group.min_len);
            let min_width_for_full_size = max_group_width + 1 + self.name.data_len;

            if terminal_width >= min_width_for_full_size + self.all_extra_width {
                let avail_width = terminal_width - min_width_for_full_size - self.all_extra_width;

                (max_group_width, self.name.data_len + avail_width, self.extra_space)
            } else {
                let avail_width = terminal_width - min_width_for_all;
                let group_width = std::cmp::min(self.group.min_len + avail_width / 2, max_group_width);
                let name_width = terminal_width - group_width - self.all_extra_width;

                (group_width, name_width, self.extra_space)
            }
        }
    }

    /// Gets header text without group column
    fn get_compact_text(&self, name_width: usize) -> String {
        format!(
            " {1:<0$} {2} {3:>6} ",
            name_width - 1,
            self.name.name,
            self.extra_columns_text,
            self.age.name
        )
    }

    /// Gets header text with group column
    fn get_full_text(&self, group_width: usize, name_width: usize) -> String {
        format!(
            " {1:<0$} {3:<2$} {4} {5:>6} ",
            group_width - 1,
            self.group.name,
            name_width,
            self.name.name,
            self.extra_columns_text,
            self.age.name
        )
    }
}

/// Builds extra columns text
fn get_extra_columns_text(extra_columns: &Option<Box<[Column]>>) -> String {
    let Some(columns) = &extra_columns else {
        return String::new();
    };

    let header_len = columns.iter().map(|c| c.max_len + 2).sum::<usize>();
    let mut header_text = String::with_capacity(header_len);
    for (i, column) in columns.iter().enumerate() {
        if i > 0 {
            header_text.push(' ');
        }

        header_text.push_str(&add_padding(
            column.name,
            column.data_len.clamp(column.min_len, column.max_len),
            column.to_right,
        ));
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

    if columns.len() > 0 && columns[0].to_right && columns[0].min_len > columns[0].data_len {
        columns[0].min_len - columns[0].data_len
    } else {
        0
    }
}
