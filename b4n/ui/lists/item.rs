use b4n_list::{FilterContext, Filterable, Item, Row};
use b4n_common::truncate;

use crate::ui::{
    ViewType,
    lists::{AGE_COLUMN_WIDTH, Header},
};

#[cfg(test)]
#[path = "./item.tests.rs"]
mod item_tests;

pub trait ItemExt {
    /// Builds and returns the whole row of values for this item.
    fn get_text(&self, view: ViewType, header: &Header, width: usize, namespace_width: usize, name_width: usize) -> String;
}

impl<T: Row + Filterable<Fc>, Fc: FilterContext> ItemExt for Item<T, Fc> {
    fn get_text(&self, view: ViewType, header: &Header, width: usize, namespace_width: usize, name_width: usize) -> String {
        let mut row = String::with_capacity(width + 2);
        match view {
            ViewType::Name => row.push_cell(self.data.name(), width, false),
            ViewType::Compact => get_compact_text(self, &mut row, header, name_width),
            ViewType::Full => get_full_text(self, &mut row, header, namespace_width, name_width),
        }

        if row.chars().count() > width {
            truncate(row.as_str(), width).to_owned()
        } else {
            row
        }
    }
}

fn get_compact_text<T: Row + Filterable<Fc>, Fc: FilterContext>(
    item: &Item<T, Fc>,
    row: &mut String,
    header: &Header,
    name_width: usize,
) {
    row.push_cell(item.data.name(), name_width, false);
    row.push(' ');
    push_inner_text(item, row, header);
    row.push(' ');
    row.push_cell(
        item.data
            .creation_timestamp()
            .map(crate::kubernetes::utils::format_datetime)
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
    namespace_width: usize,
    name_width: usize,
) {
    row.push_cell(item.data.column_text(0).as_ref(), namespace_width, false);
    row.push(' ');
    get_compact_text(item, row, header, name_width);
}

fn push_inner_text<T: Row + Filterable<Fc>, Fc: FilterContext>(item: &Item<T, Fc>, row: &mut String, header: &Header) {
    let Some(columns) = header.get_extra_columns() else {
        return;
    };

    for (i, _) in columns.iter().enumerate() {
        if i > 0 {
            row.push(' ');
        }

        let len = if i == 0 && columns[i].to_right {
            columns[i].data_len
        } else {
            columns[i].len()
        };

        row.push_cell(item.data.column_text(i + 2).as_ref(), len, columns[i].to_right);
    }
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
            (0..padding_len).for_each(|_| self.push(' '));
        }

        self.push_str(truncate(s, len));

        if !to_right && padding_len > 0 {
            (0..padding_len).for_each(|_| self.push(' '));
        }
    }
}
