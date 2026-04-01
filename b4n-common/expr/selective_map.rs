use std::collections::{HashMap, HashSet};

use crate::expr::EvaluationSource;

/// A map of categorized string lists for selective expression evaluation.
#[derive(Debug, Default, Clone)]
pub struct SelectiveMap {
    map: HashMap<&'static str, Vec<String>>,
    explicit_only: HashSet<&'static str>,
    optional: HashSet<&'static str>,
}

impl SelectiveMap {
    /// Inserts a key-value list. The key is searchable in unprefixed matches.
    pub fn with(mut self, key: &'static str, values: Vec<String>) -> Self {
        self.insert(key, values);
        self
    }

    /// Inserts a key-value list and marks it as explicit-only.\
    /// **Note** that this key will not be searched during unprefixed matching.
    pub fn with_explicit(mut self, key: &'static str, values: Vec<String>) -> Self {
        self.insert_explicit(key, values);
        self
    }

    /// Inserts a key-value list and marks it as optional.\
    /// **Note** that if this key is absent from the map, `contains_in_key` returns `true`.
    pub fn with_optional(mut self, key: &'static str, values: Vec<String>) -> Self {
        self.insert_optional(key, values);
        self
    }

    /// Inserts a key-value list. The key is searchable in unprefixed matches.
    pub fn insert(&mut self, key: &'static str, values: Vec<String>) -> &mut Self {
        self.explicit_only.remove(key);
        self.map.insert(key, values);
        self
    }

    /// Inserts a key-value list and marks it as explicit-only.\
    /// This key will **not** be searched during unprefixed matching.
    pub fn insert_explicit(&mut self, key: &'static str, values: Vec<String>) -> &mut Self {
        self.map.insert(key, values);
        self.explicit_only.insert(key);
        self
    }

    /// Inserts a key-value list and marks it as optional.
    pub fn insert_optional(&mut self, key: &'static str, values: Vec<String>) -> &mut Self {
        self.map.insert(key, values);
        self.optional.insert(key);
        self
    }

    /// Marks an existing key as optional.
    pub fn set_optional(&mut self, key: &'static str) -> &mut Self {
        self.optional.insert(key);
        self
    }

    /// Removes the optional mark from a key.
    pub fn set_required(&mut self, key: &'static str) -> &mut Self {
        self.optional.remove(key);
        self
    }

    /// Marks an existing key as explicit-only.
    pub fn set_explicit(&mut self, key: &'static str) -> &mut Self {
        self.explicit_only.insert(key);
        self
    }

    /// Removes the explicit-only mark from a key.
    pub fn set_implicit(&mut self, key: &'static str) -> &mut Self {
        self.explicit_only.remove(key);
        self
    }

    /// Returns `true` if the key is marked as explicit-only.
    pub fn is_explicit(&self, key: &str) -> bool {
        self.explicit_only.contains(key)
    }

    /// Returns `true` if the key is marked as optional.
    pub fn is_optional(&self, key: &str) -> bool {
        self.optional.contains(key)
    }
}

impl EvaluationSource for SelectiveMap {
    fn contains_in_key(&self, key: &str, value: &str) -> bool {
        match self.map.get(key) {
            Some(items) => items.iter().any(|s| s.contains(value)),
            None => self.optional.contains(key), // true if optional, false otherwise
        }
    }

    fn contains_in_any(&self, value: &str) -> bool {
        self.map
            .iter()
            .filter(|(k, _)| !self.explicit_only.contains(*k))
            .any(|(_, items)| items.iter().any(|s| s.contains(value)))
    }
}
