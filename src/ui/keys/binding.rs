use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self},
    ser::SerializeMap,
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use crate::ui::{KeyCombination, KeyCommand};

#[cfg(test)]
#[path = "./binding.tests.rs"]
mod binding_tests;

/// Key bindings for the UI.
#[derive(Debug, PartialEq, Clone)]
pub struct KeyBindings {
    bindings: HashMap<KeyCombination, HashSet<KeyCommand>>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self::empty()
            .with("Ctrl+C", KeyCommand::ApplicationExit)
            .with("Enter", KeyCommand::NavigateInto)
            .with("Esc", KeyCommand::NavigateBack)
            .with("Space", KeyCommand::NavigateSelect)
            .with("Ctrl+Space", KeyCommand::NavigateInvertSelection)
            .with("Tab", KeyCommand::NavigateComplete)
            .with("Ctrl+D", KeyCommand::NavigateDelete)
            .with("Ctrl+N", KeyCommand::MouseSupportToggle)
            .with("C", KeyCommand::ContentCopy)
            .with("E", KeyCommand::EventsShow)
            .with("I", KeyCommand::InvolvedObjectShow)
            .with("Left", KeyCommand::SelectorLeft)
            .with("Right", KeyCommand::SelectorRight)
            .with("Shift+:", KeyCommand::CommandPaletteOpen)
            .with(":", KeyCommand::CommandPaletteOpen)
            .with("Shift+>", KeyCommand::CommandPaletteOpen)
            .with(">", KeyCommand::CommandPaletteOpen)
            .with("Esc", KeyCommand::CommandPaletteReset)
            .with("Shift+/", KeyCommand::FilterOpen)
            .with("/", KeyCommand::FilterOpen)
            .with("Esc", KeyCommand::FilterReset)
            .with("Shift+/", KeyCommand::SearchOpen)
            .with("/", KeyCommand::SearchOpen)
            .with("Esc", KeyCommand::SearchReset)
            .with("N", KeyCommand::MatchNext)
            .with("P", KeyCommand::MatchPrevious)
            .with("Y", KeyCommand::YamlOpen)
            .with("X", KeyCommand::YamlDecode)
            .with("I", KeyCommand::YamlEdit)
            .with("L", KeyCommand::LogsOpen)
            .with("T", KeyCommand::LogsTimestamps)
            .with("P", KeyCommand::PreviousLogsOpen)
            .with("S", KeyCommand::ShellOpen)
            .with("Esc", KeyCommand::ShellEscape)
            .with("Ctrl+F", KeyCommand::PortForwardsOpen)
            .with("F", KeyCommand::PortForwardsCreate)
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
        let result = KeyBindings::default();
        if let Some(other) = other {
            merge(result, other)
        } else {
            result
        }
    }

    /// Inserts the given key `combination` and associated `command` into the [`KeyBindings`],
    /// returning the updated instance.
    pub fn with(mut self, combination: &str, command: KeyCommand) -> Self {
        self.bindings.entry(combination.into()).or_default().insert(command);
        self
    }

    /// Returns `true` if the given [`KeyCombination`] is bound to the specified [`KeyCommand`].
    pub fn has_binding(&self, key: &KeyCombination, command: KeyCommand) -> bool {
        if let Some(commands) = self.bindings.get(key) {
            commands.contains(&command)
        } else {
            false
        }
    }

    /// Returns the [`KeyCombination`] associated with the specified [`KeyCommand`].
    pub fn get_key(&self, command: KeyCommand) -> Option<KeyCombination> {
        self.bindings
            .iter()
            .find(|(_, commands)| commands.contains(&command))
            .map(|(combination, _)| combination)
            .copied()
    }
}

impl Serialize for KeyBindings {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut inverted: HashMap<String, Vec<String>> = HashMap::new();
        for (combination, commands) in &self.bindings {
            for command in commands {
                inverted.entry(command.to_string()).or_default().push(combination.to_string());
            }
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

        let mut bindings: HashMap<KeyCombination, HashSet<KeyCommand>> = HashMap::new();
        for (command_str, combination_str) in map {
            let command = KeyCommand::from_str(&command_str)
                .map_err(|_| de::Error::custom(format_args!("invalid command: {command_str}")))?;

            for combination in combination_str.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let key_combination = KeyCombination::from_str(combination)
                    .map_err(|_| de::Error::custom(format_args!("invalid key combination: {combination}")))?;
                bindings.entry(key_combination).or_default().insert(command);
            }
        }

        Ok(KeyBindings { bindings })
    }
}

fn merge(left: KeyBindings, right: KeyBindings) -> KeyBindings {
    let mut result = invert(left);
    for (command, combinations) in invert(right) {
        result.insert(command, combinations);
    }

    let mut bindings: HashMap<KeyCombination, HashSet<KeyCommand>> = HashMap::new();
    for (command, combinations) in result {
        for combination in combinations {
            bindings.entry(combination).or_default().insert(command);
        }
    }

    KeyBindings { bindings }
}

fn invert(bindings: KeyBindings) -> HashMap<KeyCommand, HashSet<KeyCombination>> {
    let mut inverted: HashMap<KeyCommand, HashSet<KeyCombination>> = HashMap::new();
    for (combination, commands) in bindings.bindings {
        for command in commands {
            inverted.entry(command).or_default().insert(combination);
        }
    }

    inverted
}
