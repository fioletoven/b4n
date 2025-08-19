use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display};
use std::str::FromStr;

#[cfg(test)]
#[path = "./command.tests.rs"]
mod command_tests;

/// Possible errors from [`KeyCommand`] parsing.
#[derive(thiserror::Error, Debug)]
pub enum KeyCommandError {
    /// Unknown key binding command.
    #[error("unknown key binding command")]
    UnknownCommand,
}

/// Defines what part of the UI the command targets.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum KeyCommand {
    ApplicationExit,
    NavigateInto,
    NavigateBack,
    NavigateSelect,
    NavigateInvertSelection,
    CommandPaletteOpen,
    CommandPaletteReset,
    FilterOpen,
    FilterReset,
    SearchOpen,
    SearchReset,
    ResourcesDelete,
    YamlOpen,
    YamlDecode,
    PreviousLogsOpen,
    LogsOpen,
    ShellOpen,
    ShellEscape,
    PortForwardsOpen,
    PortForwardsCreate,
}

impl Display for KeyCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyCommand::ApplicationExit => f.write_str("app.exit")?,
            KeyCommand::NavigateInto => f.write_str("navigate.into")?,
            KeyCommand::NavigateBack => f.write_str("navigate.back")?,
            KeyCommand::NavigateSelect => f.write_str("navigate.select")?,
            KeyCommand::NavigateInvertSelection => f.write_str("navigate.invert-selection")?,
            KeyCommand::CommandPaletteOpen => f.write_str("command-palette.open")?,
            KeyCommand::CommandPaletteReset => f.write_str("command-palette.close")?,
            KeyCommand::FilterOpen => f.write_str("filter.open")?,
            KeyCommand::FilterReset => f.write_str("filter.reset")?,
            KeyCommand::SearchOpen => f.write_str("search.open")?,
            KeyCommand::SearchReset => f.write_str("search.reset")?,
            KeyCommand::ResourcesDelete => f.write_str("resources.delete")?,
            KeyCommand::YamlOpen => f.write_str("yaml.open")?,
            KeyCommand::YamlDecode => f.write_str("yaml.decode")?,
            KeyCommand::PreviousLogsOpen => f.write_str("previous-logs.open")?,
            KeyCommand::LogsOpen => f.write_str("logs.open")?,
            KeyCommand::ShellOpen => f.write_str("shell.open")?,
            KeyCommand::ShellEscape => f.write_str("shell.escape")?,
            KeyCommand::PortForwardsOpen => f.write_str("port-forwards.open")?,
            KeyCommand::PortForwardsCreate => f.write_str("port-forwards.create")?,
        }
        Ok(())
    }
}

impl FromStr for KeyCommand {
    type Err = KeyCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "app.exit" => Ok(KeyCommand::ApplicationExit),
            "navigate.into" => Ok(KeyCommand::NavigateInto),
            "navigate.back" => Ok(KeyCommand::NavigateBack),
            "navigate.select" => Ok(KeyCommand::NavigateSelect),
            "navigate.invert-selection" => Ok(KeyCommand::NavigateInvertSelection),
            "command-palette.open" => Ok(KeyCommand::CommandPaletteOpen),
            "command-palette.close" => Ok(KeyCommand::CommandPaletteReset),
            "filter.open" => Ok(KeyCommand::FilterOpen),
            "filter.reset" => Ok(KeyCommand::FilterReset),
            "search.open" => Ok(KeyCommand::SearchOpen),
            "search.reset" => Ok(KeyCommand::SearchReset),
            "resources.delete" => Ok(KeyCommand::ResourcesDelete),
            "yaml.open" => Ok(KeyCommand::YamlOpen),
            "yaml.decode" => Ok(KeyCommand::YamlDecode),
            "logs.open" => Ok(KeyCommand::LogsOpen),
            "previous-logs.open" => Ok(KeyCommand::PreviousLogsOpen),
            "shell.open" => Ok(KeyCommand::ShellOpen),
            "shell.escape" => Ok(KeyCommand::ShellEscape),
            "port-forwards.open" => Ok(KeyCommand::PortForwardsOpen),
            "port-forwards.create" => Ok(KeyCommand::PortForwardsCreate),
            _ => Err(KeyCommandError::UnknownCommand),
        }
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
