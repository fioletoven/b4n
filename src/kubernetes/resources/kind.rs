use crate::{app::lists::Row, utils::truncate};

/// Represents kubernetes kind
pub struct Kind {
    pub uid: Option<String>,
    pub name: String,
}

impl Kind {
    /// Creates new [`Kind`] instance
    pub fn new(name: &str) -> Self {
        Self {
            uid: Some(format!("_{}_", name)),
            name: name.to_owned(),
        }
    }

    /// Returns `UID` of this kubernetes kind
    pub fn get_uid(&self) -> Option<&str> {
        self.uid.as_deref()
    }
}

impl Row for Kind {
    fn group(&self) -> &str {
        ""
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> &str {
        if column == 1 {
            self.name.as_str()
        } else {
            "n/a"
        }
    }
}
