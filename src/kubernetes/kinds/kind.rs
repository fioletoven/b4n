use std::borrow::Cow;

use crate::{
    kubernetes::{CORE_VERSION, Kind},
    ui::lists::{BasicFilterContext, Filterable, Row},
    utils::{truncate, truncate_left},
};

/// Represents kubernetes kind.
#[derive(Clone)]
pub struct KindItem {
    pub kind: Kind,
    pub multiple_groups: bool,
    pub multiple_versions: bool,
}

impl KindItem {
    /// Creates new [`KindItem`] instance.
    pub fn new(group: &str, name: String, version: &str) -> Self {
        let kind: Kind = if group.is_empty() && version == CORE_VERSION {
            name.into()
        } else {
            format!("{name}.{group}/{version}").into()
        };

        Self {
            kind,
            multiple_groups: false,
            multiple_versions: false,
        }
    }

    pub fn with_multiple_groups(mut self, has_multiple_groups: bool) -> Self {
        self.multiple_groups = has_multiple_groups;
        self
    }

    /// Returns full `name` of the item respecting provided `width` and truncating start if needed.
    pub fn get_name_end(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate_left(self.name(), width))
    }
}

impl Row for KindItem {
    fn uid(&self) -> &str {
        self.kind.as_str()
    }

    fn group(&self) -> &str {
        self.kind.group()
    }

    fn name(&self) -> &str {
        if self.multiple_versions {
            self.kind.as_str()
        } else if self.multiple_groups {
            self.kind.name_and_group()
        } else {
            self.kind.name()
        }
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name(), width))
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        Cow::Borrowed(match column {
            0 => self.group(),
            1 => self.name(),
            2 => self.kind.version(),
            _ => "n/a",
        })
    }

    fn column_sort_text(&self, column: usize) -> &str {
        match column {
            0 => self.group(),
            1 => self.name(),
            2 => self.kind.version(),
            _ => "n/a",
        }
    }
}

impl Filterable<BasicFilterContext> for KindItem {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.name().contains(&context.pattern)
    }
}
