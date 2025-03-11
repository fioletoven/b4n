use crate::{
    app::lists::{BasicFilterContext, Filterable, Row},
    utils::{add_padding, truncate},
};

/// Filter pattern item.
#[derive(Default)]
pub struct Pattern {
    pub value: String,
}

impl From<String> for Pattern {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl Row for Pattern {
    fn uid(&self) -> Option<&str> {
        Some(&self.value)
    }

    fn group(&self) -> &str {
        "n/a"
    }

    fn name(&self) -> &str {
        &self.value
    }

    fn get_name(&self, width: usize) -> String {
        add_padding(&self.value, width)
    }

    fn get_name_for_highlighted(&self, width: usize) -> String {
        let width = width.saturating_sub(14);
        format!("{1:<0$} [TAB to insert]", width, truncate(&self.value, width))
    }

    fn column_text(&self, column: usize) -> &str {
        match column {
            0 => "n/a",
            1 => &self.value,
            _ => "n/a",
        }
    }

    fn column_sort_text(&self, column: usize) -> &str {
        self.column_text(column)
    }
}

impl Filterable<BasicFilterContext> for Pattern {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.contains(&context.pattern)
    }
}
