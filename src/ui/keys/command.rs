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
    NavigateComplete,
    NavigateDelete,
    MouseSupportToggle,
    ContentCopy,
    EventsShow,
    SelectorLeft,
    SelectorRight,
    CommandPaletteOpen,
    CommandPaletteReset,
    FilterOpen,
    FilterReset,
    SearchOpen,
    SearchReset,
    MatchNext,
    MatchPrevious,
    YamlOpen,
    YamlDecode,
    PreviousLogsOpen,
    LogsOpen,
    LogsTimestamps,
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
            KeyCommand::NavigateComplete => f.write_str("navigate.complete")?,
            KeyCommand::NavigateDelete => f.write_str("navigate.delete")?,
            KeyCommand::MouseSupportToggle => f.write_str("mouse-support.toggle")?,
            KeyCommand::ContentCopy => f.write_str("content.copy")?,
            KeyCommand::EventsShow => f.write_str("events.show")?,
            KeyCommand::SelectorLeft => f.write_str("selector.left")?,
            KeyCommand::SelectorRight => f.write_str("selector.right")?,
            KeyCommand::CommandPaletteOpen => f.write_str("command-palette.open")?,
            KeyCommand::CommandPaletteReset => f.write_str("command-palette.close")?,
            KeyCommand::FilterOpen => f.write_str("filter.open")?,
            KeyCommand::FilterReset => f.write_str("filter.reset")?,
            KeyCommand::SearchOpen => f.write_str("search.open")?,
            KeyCommand::SearchReset => f.write_str("search.reset")?,
            KeyCommand::MatchNext => f.write_str("match.next")?,
            KeyCommand::MatchPrevious => f.write_str("match.previous")?,
            KeyCommand::YamlOpen => f.write_str("yaml.open")?,
            KeyCommand::YamlDecode => f.write_str("yaml.decode")?,
            KeyCommand::PreviousLogsOpen => f.write_str("previous-logs.open")?,
            KeyCommand::LogsOpen => f.write_str("logs.open")?,
            KeyCommand::LogsTimestamps => f.write_str("logs.timestamps")?,
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
            "navigate.complete" => Ok(KeyCommand::NavigateComplete),
            "navigate.delete" => Ok(KeyCommand::NavigateDelete),
            "mouse-support.toggle" => Ok(KeyCommand::MouseSupportToggle),
            "content.copy" => Ok(KeyCommand::ContentCopy),
            "events.show" => Ok(KeyCommand::EventsShow),
            "selector.left" => Ok(KeyCommand::SelectorLeft),
            "selector.right" => Ok(KeyCommand::SelectorRight),
            "command-palette.open" => Ok(KeyCommand::CommandPaletteOpen),
            "command-palette.close" => Ok(KeyCommand::CommandPaletteReset),
            "filter.open" => Ok(KeyCommand::FilterOpen),
            "filter.reset" => Ok(KeyCommand::FilterReset),
            "search.open" => Ok(KeyCommand::SearchOpen),
            "search.reset" => Ok(KeyCommand::SearchReset),
            "match.next" => Ok(KeyCommand::MatchNext),
            "match.previous" => Ok(KeyCommand::MatchPrevious),
            "yaml.open" => Ok(KeyCommand::YamlOpen),
            "yaml.decode" => Ok(KeyCommand::YamlDecode),
            "logs.open" => Ok(KeyCommand::LogsOpen),
            "logs.timestamps" => Ok(KeyCommand::LogsTimestamps),
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
