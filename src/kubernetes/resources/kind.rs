use crate::{
    app::lists::{BasicFilterContext, Filterable, Row},
    utils::truncate,
};

/// Represents kubernetes kind.
pub struct Kind {
    pub uid: Option<String>,
    pub group: String,
    pub name: String,
    pub full_name: String,
    pub version: String,
    pub multiple: bool,
}

impl Kind {
    /// Creates new [`Kind`] instance.
    pub fn new(group: String, name: String, version: String) -> Self {
        let full_name = if group.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", name, group)
        };

        Self {
            uid: Some(format!("_{}:{}:{}_", group, name, version)),
            group,
            name,
            full_name,
            version,
            multiple: false,
        }
    }
}

impl Row for Kind {
    fn uid(&self) -> Option<&str> {
        self.uid.as_deref()
    }

    fn group(&self) -> &str {
        &self.group
    }

    fn name(&self) -> &str {
        if self.multiple { &self.full_name } else { &self.name }
    }

    fn get_name(&self, width: usize) -> String {
        if self.multiple {
            format!("{1:<0$}", width, truncate(self.full_name.as_str(), width))
        } else {
            format!("{1:<0$}", width, truncate(self.name.as_str(), width))
        }
    }

    fn column_text(&self, column: usize) -> &str {
        match column {
            0 => &self.group,
            1 => self.name(),
            2 => &self.version,
            _ => "n/a",
        }
    }
}

impl Filterable<BasicFilterContext> for Kind {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.name.contains(&context.pattern)
    }
}
