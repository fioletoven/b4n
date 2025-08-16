use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
};
use std::{collections::HashMap, fmt};

use crate::ui::{KeyCombination, KeyCommand};

#[cfg(test)]
#[path = "./binding.tests.rs"]
mod binding_tests;

/// Key bindings for the UI.
#[derive(Debug, PartialEq)]
pub struct KeyBindings {
    bindings: HashMap<KeyCombination, KeyCommand>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut result = Self {
            bindings: HashMap::new(),
        };

        result.insert("Ctrl+C", "exit-app");
        result.insert("Shift+:", "command-palette.open");
        result.insert("Shift+>", "command-palette.open");
        result.insert("/", "filter.open");

        result
    }
}

impl KeyBindings {
    /// Creates new [`KeyBindings`] instance.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Adds new or updates key binding to the list.
    pub fn insert(&mut self, combination: &str, command: &str) {
        self.bindings.insert(combination.into(), command.into());
    }

    /// Merges `other` key bindings sequence with this one.
    pub fn merge(&mut self, other: KeyBindings) {
        for (combination, command) in other.bindings {
            self.bindings.insert(combination, command);
        }
    }
}

impl Serialize for KeyBindings {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Binding<'a> {
            command: &'a KeyCommand,
            combination: &'a KeyCombination,
        }

        let mut result = serializer.serialize_seq(Some(self.bindings.len()))?;

        for (combination, command) in &self.bindings {
            result.serialize_element(&Binding { command, combination })?;
        }

        result.end()
    }
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Binding {
            command: KeyCommand,
            combination: KeyCombination,
        }

        struct KeyBindingsVisitor;

        impl<'de> Visitor<'de> for KeyBindingsVisitor {
            type Value = KeyBindings;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of {command, combination} objects")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut bindings = HashMap::new();

                while let Some(binding) = seq.next_element::<Binding>()? {
                    bindings.insert(binding.combination, binding.command);
                }

                Ok(KeyBindings { bindings })
            }
        }

        deserializer.deserialize_seq(KeyBindingsVisitor)
    }
}
