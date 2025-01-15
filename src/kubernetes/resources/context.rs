use crate::{app::lists::Row, utils::truncate};

/// Kubernetes context.
pub struct Context {
    pub uid: Option<String>,
    pub group: String,
    pub name: String,
    pub cluster: Option<String>,
}

impl Context {
    /// Creates new [`Context`] instance.
    pub fn new(name: String, cluster: Option<String>) -> Self {
        Self {
            uid: Some(format!("_{}:{}_", name, cluster.as_deref().unwrap_or_default())),
            group: "context".to_owned(),
            name,
            cluster,
        }
    }
}

impl Row for Context {
    fn uid(&self) -> Option<&str> {
        self.uid.as_deref()
    }

    fn group(&self) -> &str {
        &self.group
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> &str {
        match column {
            0 => &self.group,
            1 => &self.name,
            2 => self.cluster.as_deref().unwrap_or_default(),
            _ => "n/a",
        }
    }
}
