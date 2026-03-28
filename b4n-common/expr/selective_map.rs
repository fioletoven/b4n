use std::collections::{HashMap, HashSet};

use crate::expr::EvaluationSource;

/// A map of categorized string lists for selective expression evaluation.
#[derive(Debug, Clone)]
pub struct SelectiveMap {
    map: HashMap<&'static str, Vec<String>>,
    explicit_only: HashSet<&'static str>,
}

impl Default for SelectiveMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectiveMap {
    /// Creates a new empty [`SelectiveMap`].
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            explicit_only: HashSet::new(),
        }
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
}

impl EvaluationSource for SelectiveMap {
    fn contains_in_key(&self, key: &str, value: &str) -> bool {
        self.map
            .get(key)
            .map(|items| items.iter().any(|s| s.contains(value)))
            .unwrap_or(false)
    }

    fn contains_in_any(&self, value: &str) -> bool {
        self.map
            .iter()
            .filter(|(k, _)| !self.explicit_only.contains(*k))
            .any(|(_, items)| items.iter().any(|s| s.contains(value)))
    }
}
