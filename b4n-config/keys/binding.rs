use serde::de::{self, Unexpected, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use crate::define_key_commands;
use crate::keys::KeyCombination;

#[cfg(test)]
#[path = "./binding.tests.rs"]
mod binding_tests;

/// Possible errors from [`KeyCommand`] parsing.
#[derive(thiserror::Error, Debug)]
pub enum KeyCommandError {
    /// Unknown key binding command.
    #[error("unknown key binding command")]
    UnknownCommand,
}

define_key_commands! {
    bindings = KeyBindings;

    /// Defines what part of the UI the command targets.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum KeyCommand {
        ApplicationExit => "app.exit" @ "Ctrl+C",
        CommandPaletteOpen => "command-palette.open" @ ":", ">", "Shift+:", "Shift+>",
        CommandPaletteReset => "command-palette.close" @ "Esc",
        ContentCopy => "content.copy" @ "C",
        EventsShow => "events.show" @ "E",
        FilterOpen => "filter.open" @ "/", "Shift+/",
        FilterReset => "filter.reset" @ "Esc",
        HistoryOpen => "history.open" @ "H",
        InvolvedObjectShow => "involved-object.show" @ "I",
        LogsOpen => "logs.open" @ "L",
        LogsTimestamps => "logs.timestamps" @ "T",
        MatchNext => "match.next" @ "N",
        MatchPrevious => "match.previous" @ "P",
        MouseMenuOpen => "mouse-menu.open" @ "M",
        MouseSupportToggle => "mouse-support.toggle" @ "Ctrl+N", "Ctrl+M",
        NavigateBack => "navigate.back" @ "Esc",
        NavigateComplete => "navigate.complete" @ "Tab",
        NavigateDelete => "navigate.delete" @ "Ctrl+D",
        NavigateInto => "navigate.into" @ "Enter",
        NavigateInvertSelection => "navigate.invert-selection" @ "Ctrl+Space",
        NavigateSelect => "navigate.select" @ "Space",
        NavigateSelectAll => "navigate.select-all" @ "Ctrl+A",
        PortForwardsCreate => "port-forwards.create" @ "F",
        PortForwardsOpen => "port-forwards.open" @ "Ctrl+F",
        PortForwardsCleanup => "port-forwards.cleanup" @ "Ctrl+R",
        PreviousLogsOpen => "previous-logs.open" @ "P",
        SearchOpen => "search.open" @ "/", "Shift+/",
        SearchReset => "search.reset" @ "Esc",
        SelectorLeft => "selector.left" @ "Left",
        SelectorRight => "selector.right" @ "Right",
        ShellEscape => "shell.escape" @ "Esc",
        ShellOpen => "shell.open" @ "S",
        YamlCreate => "yaml.create" @ "N",
        YamlDecode => "yaml.decode" @ "X",
        YamlEdit => "yaml.edit" @ "I",
        YamlOpen => "yaml.open" @ "Y",
    }
}

impl Serialize for KeyCommand {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeyCommand {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyCommandVisitor;

        impl Visitor<'_> for KeyCommandVisitor {
            type Value = KeyCommand;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string containing key command")
            }

            fn visit_str<E>(self, value: &str) -> Result<KeyCommand, E>
            where
                E: de::Error,
            {
                match KeyCommand::from_str(value) {
                    Ok(command) => Ok(command),
                    Err(_) => Err(de::Error::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_str(KeyCommandVisitor)
    }
}

/// Key bindings for the UI.
#[derive(Debug, PartialEq, Clone)]
pub struct KeyBindings {
    bindings: HashMap<KeyCombination, HashSet<KeyCommand>>,
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

    /// Returns inverted hash map with key commands and theirs key combinations.
    pub fn inverted(&self) -> HashMap<KeyCommand, HashSet<KeyCombination>> {
        let mut inverted: HashMap<KeyCommand, HashSet<KeyCombination>> = HashMap::new();
        for (combination, commands) in &self.bindings {
            for command in commands {
                inverted.entry(*command).or_default().insert(*combination);
            }
        }

        inverted
    }

    /// Returns `true` if the given [`KeyCombination`] is bound to the specified [`KeyCommand`].
    pub fn has_binding(&self, key: &KeyCombination, command: KeyCommand) -> bool {
        if let Some(commands) = self.bindings.get(key) {
            commands.contains(&command)
        } else {
            false
        }
    }

    /// Returns the first [`KeyCombination`] name associated with the specified [`KeyCommand`].
    pub fn get_key_name(&self, command: KeyCommand) -> Option<String> {
        let mut keys = self
            .bindings
            .iter()
            .filter(|(_, commands)| commands.contains(&command))
            .map(|(combination, _)| combination.to_string())
            .collect::<Vec<_>>();
        keys.sort();

        if keys.is_empty() { None } else { Some(keys.swap_remove(0)) }
    }
}

impl Serialize for KeyBindings {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let inverted = self
            .inverted()
            .into_iter()
            .map(|(command, combinations)| {
                let mut combinations = combinations.iter().map(ToString::to_string).collect::<Vec<_>>();
                combinations.sort();
                (command.to_string(), combinations.join(", "))
            })
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
