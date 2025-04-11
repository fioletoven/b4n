/// Represents kubernetes kind together with its group.  
/// **Note** that it can be also used for plural names.
#[derive(Default)]
pub struct Kind {
    kind: String,
    index: Option<usize>,
}

impl Kind {
    /// Creates new [`Kind`] instance.
    pub fn new(kind: &str, group: &str) -> Self {
        let kind = if group.is_empty() {
            kind.to_owned()
        } else {
            format!("{}.{}", kind, group)
        };
        let index = kind.find('.');
        Self { kind, index }
    }

    /// Creates new [`Kind`] instance from string.
    pub fn from(kind: impl Into<String>) -> Self {
        let kind = kind.into();
        let index = kind.find('.');
        Self { kind, index }
    }

    /// Returns `true` if kind has group.
    pub fn has_group(&self) -> bool {
        self.index.is_some()
    }

    /// Returns kind as string slice.
    pub fn as_str(&self) -> &str {
        &self.kind
    }

    /// Returns kind name.
    pub fn name(&self) -> &str {
        if let Some(index) = self.index {
            &self.kind[..index]
        } else {
            &self.kind
        }
    }

    /// Return kind group.
    pub fn group(&self) -> &str {
        if let Some(index) = self.index {
            &self.kind[index + 1..]
        } else {
            ""
        }
    }
}

impl From<String> for Kind {
    fn from(value: String) -> Self {
        let index = value.find('.');
        Self { kind: value, index }
    }
}
