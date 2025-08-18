use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self},
    ser::SerializeMap,
};
use std::{collections::HashMap, str::FromStr};

use crate::ui::{CommandAction, CommandTarget, KeyCombination, KeyCommand};

#[cfg(test)]
#[path = "./binding.tests.rs"]
mod binding_tests;

pub const COMMAND_APP_EXIT: KeyCommand = KeyCommand::new(CommandTarget::Application, CommandAction::Exit);

/// Key bindings for the UI.
#[derive(Debug, PartialEq, Clone)]
pub struct KeyBindings {
    bindings: HashMap<KeyCombination, KeyCommand>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self::empty()
            .with("Ctrl+C", "app.exit")
            .with("Shift+:", "command-palette.open")
            .with("Shift+>", "command-palette.open")
            .with("/", "filter.open")
    }
}

impl KeyBindings {
    /// Creates empty [`KeyBindings`] instance.
    pub fn empty() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Creates default [`KeyBindings`] instance updated with `other` key bindings sequence.
    pub fn default_with(other: Option<KeyBindings>) -> Self {
        let mut result = KeyBindings::default();
        if let Some(other) = other {
            for (combination, command) in other.bindings {
                result.bindings.insert(combination, command);
            }
        }

        result
    }

    /// Inserts the given key `combination` and associated `command` into the [`KeyBindings`],
    /// returning the updated instance.
    pub fn with(mut self, combination: &str, command: &str) -> Self {
        self.bindings.insert(combination.into(), command.into());
        self
    }

    /// Returns `true` if the given [`KeyCombination`] is bound to the specified [`KeyCommand`].
    pub fn has_binding(&self, key: &KeyCombination, command: &KeyCommand) -> bool {
        if let Some(supposed) = self.bindings.get(key) {
            supposed.target == command.target && supposed.action == command.action
        } else {
            false
        }
    }

    /// Returns the [`KeyCombination`] associated with the specified [`KeyCommand`].
    pub fn get_key(&self, command: &KeyCommand) -> Option<KeyCombination> {
        self.bindings
            .iter()
            .find(|(_, cmd)| *cmd == command)
            .map(|(combination, _)| combination)
            .copied()
    }
}

impl Serialize for KeyBindings {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut inverted: HashMap<String, Vec<String>> = HashMap::new();
        for (combination, command) in &self.bindings {
            inverted.entry(command.to_string()).or_default().push(combination.to_string());
        }

        let inverted = inverted
            .into_iter()
            .map(|(command, combinations)| (command, combinations.join(", ")))
            .collect::<HashMap<_, _>>();

        let mut keys = inverted.keys().collect::<Vec<_>>();
        keys.sort();

        let mut map = serializer.serialize_map(Some(inverted.len()))?;
        for key in keys {
            if let Some(value) = inverted.get(key) {
                map.serialize_entry(key, value)?;
            }
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;

        let mut bindings = HashMap::new();
        for (command_str, combination_str) in map {
            let command = KeyCommand::from_str(&command_str)
                .map_err(|_| de::Error::custom(format_args!("invalid command: {}", command_str)))?;

            for combination in combination_str.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let key_combination = KeyCombination::from_str(combination)
                    .map_err(|_| de::Error::custom(format_args!("invalid key combination: {}", combination)))?;
                bindings.insert(key_combination, command.clone());
            }
        }

        Ok(KeyBindings { bindings })
    }
}
