use b4n_common::{substring_owned, truncate};
use b4n_list::{FilterContext, Filterable, Item, Row};

use crate::table::{AGE_COLUMN_WIDTH, Header, ViewType, header::HeaderWidths};
use crate::utils::consume_and_add_space;

pub trait ItemExt {
    /// Builds and returns the whole row of values for this item.
    fn get_text(&self, view: ViewType, header: &Header, widths: &HeaderWidths, width: usize, offset: usize) -> String;
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> ItemExt for Item<T, Fc> {
    fn get_text(&self, view: ViewType, header: &Header, widths: &HeaderWidths, width: usize, offset: usize) -> String {
        let mut row = String::with_capacity(width + 2);
        match view {
            ViewType::Name => row.push_cell(self.data.name(), width, false),
            ViewType::Compact => get_compact_text(self, &mut row, header, widths),
            ViewType::Full => get_full_text(self, &mut row, header, widths),
        }

        if offset > 0 {
            substring_owned(row, offset, width)
        } else {
            row.truncate(width);
            row
        }
    }
}

fn get_compact_text<T: Row + Filterable<Fc>, Fc: FilterContext>(
    item: &Item<T, Fc>,
    row: &mut String,
    header: &Header,
    widths: &HeaderWidths,
) {
    let mut name_width = widths.name + widths.name_extra;
    let mut extra_width = widths.extra;
    if widths.extra > 0 {
        extra_width = extra_width.saturating_sub(header.double_spaces_count());
    } else {
        name_width = name_width.saturating_sub(header.double_spaces_count());
    }

    row.push_cell(item.data.name(), name_width, false);
    row.push(' ');
    push_inner_text(item, row, header, extra_width);
    row.push(' ');
    row.push_cell(
        item.data
            .creation_timestamp()
            .map(b4n_kube::utils::format_datetime)
            .as_deref()
            .unwrap_or("n/a"),
        AGE_COLUMN_WIDTH + 1,
        true,
    );
}

fn get_full_text<T: Row + Filterable<Fc>, Fc: FilterContext>(
    item: &Item<T, Fc>,
    row: &mut String,
    header: &Header,
    widths: &HeaderWidths,
) {
    row.push_cell(item.data.column_text(0).as_ref(), widths.group, false);
    row.push(' ');
    get_compact_text(item, row, header, widths);
}

fn push_inner_text<T: Row + Filterable<Fc>, Fc: FilterContext>(
    item: &Item<T, Fc>,
    row: &mut String,
    header: &Header,
    extra_space: usize,
) {
    let mut double_spaces_count = header.double_spaces_count();
    consume_and_add_space(row, &mut double_spaces_count);

    let Some(columns) = header.get_extra_columns() else {
        return;
    };

    let last = columns.len().saturating_sub(1);
    for (i, column) in columns.iter().enumerate() {
        if i > 0 {
            row.push(' ');
            consume_and_add_space(row, &mut double_spaces_count);
        }

        let len = if i == 0 && column.to_right {
            column.data_len
        } else if i == last {
            column.len() + extra_space
        } else {
            column.len()
        };

        row.push_cell(item.data.column_text(i + 2).as_ref(), len, column.to_right);
    }

    consume_and_add_space(row, &mut double_spaces_count);
}

/// Extension methods for string.
pub trait RowStringExt {
    /// Appends a given cell text onto the end of this `String`.
    fn push_cell(&mut self, s: &str, len: usize, to_right: bool);
}

impl RowStringExt for String {
    fn push_cell(&mut self, s: &str, len: usize, to_right: bool) {
        if len == 0 {
            return;
        }

        let padding_len = len.saturating_sub(s.chars().count());
        if to_right && padding_len > 0 {
            self.extend(std::iter::repeat_n(' ', padding_len));
        }

        self.push_str(truncate(s, len));

        if !to_right && padding_len > 0 {
            self.extend(std::iter::repeat_n(' ', padding_len));
        }
    }
}
