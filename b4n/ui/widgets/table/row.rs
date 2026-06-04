use b4n_common::truncate;
use b4n_list::{BasicFilterContext, Filterable, Row};
use std::borrow::Cow;

use crate::ui::widgets::table::Cell;

/// Basic table row.
pub struct BasicRow {
    uid: String,
    name: String,
    cells: Box<[Cell]>,
}

impl BasicRow {
    /// Creates a new [`BasicRow`].
    pub fn new(uid: impl Into<String>, name: impl Into<String>, cells: Box<[Cell]>) -> Self {
        let uid = uid.into();
        let name = name.into();

        Self { uid, name, cells }
    }
}

impl Row for BasicRow {
    fn uid(&self) -> &str {
        &self.uid
    }

    fn group(&self) -> &str {
        ""
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        match column {
            0 => Cow::Borrowed(self.group()),
            1 => Cow::Borrowed(self.name()),
            idx => self
                .cells
                .get(idx.saturating_sub(2))
                .map_or_else(|| Cow::Borrowed("n/a"), Cell::text),
        }
    }

    fn column_sort_text(&self, column: usize) -> &str {
        match column {
            0 => self.group(),
            1 => self.name(),
            idx => self.cells.get(idx.saturating_sub(2)).map_or("n/a", Cell::sort_text),
        }
    }
}

impl Filterable<BasicFilterContext> for BasicRow {
    fn get_context(pattern: &str, _settings: Option<&str>) -> BasicFilterContext {
        pattern.to_ascii_lowercase().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.contains(context.pattern.as_str())
    }
}
