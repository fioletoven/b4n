use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::{self, Display};
use std::str::FromStr;

#[cfg(test)]
#[path = "./command.tests.rs"]
mod command_tests;

/// Possible errors from [`KeyCommand`] parsing.
#[derive(thiserror::Error, Debug)]
pub enum KeyCommandError {
    /// Command value is missing.
    #[error("command value is missing")]
    MissingCommandValue,
}

/// Defines what part of the UI the command targets.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum CommandTarget {
    Application,
    CommandPalette,
    Filter,
    View(Cow<'static, str>),
}

impl Display for CommandTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandTarget::Application => f.write_str("app")?,
            CommandTarget::CommandPalette => f.write_str("command-palette")?,
            CommandTarget::Filter => f.write_str("filter")?,
            CommandTarget::View(v) => f.write_str(&v.to_lowercase())?,
        }
        Ok(())
    }
}

impl From<&str> for CommandTarget {
    fn from(value: &str) -> Self {
        let value = value.to_lowercase();
        match value.as_str() {
            "" | "application" | "app" => CommandTarget::Application,
            "command-palette" => CommandTarget::CommandPalette,
            "filter" => CommandTarget::Filter,
            _ => CommandTarget::View(value.into()),
        }
    }
}

impl CommandTarget {
    /// Creates a [`CommandTarget::View`] variant of the [`CommandTarget`].
    pub fn view(value: impl Into<Cow<'static, str>>) -> Self {
        CommandTarget::View(value.into())
    }
}

/// Defines what action should be performed on a target.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum CommandAction {
    Open,
    Close,
    Exit,
    Search,
    Action(Cow<'static, str>),
}

impl Display for CommandAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandAction::Open => f.write_str("open")?,
            CommandAction::Close => f.write_str("close")?,
            CommandAction::Exit => f.write_str("exit")?,
            CommandAction::Search => f.write_str("search")?,
            CommandAction::Action(a) => f.write_str(&a.to_lowercase())?,
        }
        Ok(())
    }
}

impl FromStr for CommandAction {
    type Err = KeyCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase();
        match value.as_str() {
            "" => Err(KeyCommandError::MissingCommandValue),
            "open" => Ok(CommandAction::Open),
            "close" => Ok(CommandAction::Close),
            "exit" => Ok(CommandAction::Exit),
            "search" => Ok(CommandAction::Search),
            _ => Ok(CommandAction::Action(value.into())),
        }
    }
}

impl CommandAction {
    /// Creates a [`CommandAction::Action`] variant of the [`CommandAction`].
    pub fn action(value: impl Into<Cow<'static, str>>) -> Self {
        CommandAction::Action(value.into())
    }
}

/// The UI command specification.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct KeyCommand {
    pub target: CommandTarget,
    pub action: CommandAction,
}

impl Display for KeyCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.target, self.action)
    }
}

impl From<&str> for KeyCommand {
    fn from(value: &str) -> Self {
        KeyCommand::from_str(value).unwrap_or_else(|_| KeyCommand {
            target: CommandTarget::Application,
            action: CommandAction::Action("".into()),
        })
    }
}

impl FromStr for KeyCommand {
    type Err = KeyCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.to_lowercase();
        let (kind, command) = get_command_elements(&value);
        let kind = CommandTarget::from(kind);
        match CommandAction::from_str(command) {
            Ok(command) => Ok(KeyCommand {
                target: kind,
                action: command,
            }),
            Err(error) => Err(error),
        }
    }
}

impl KeyCommand {
    /// Creates new [`KeyCommand`] instance.
    pub fn new(kind: CommandTarget, command: CommandAction) -> Self {
        Self {
            target: kind,
            action: command,
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

fn get_command_elements(value: &str) -> (&str, &str) {
    let elements = value.splitn(2, '.').collect::<Vec<_>>();
    if elements.len() == 1 {
        ("", elements[0])
    } else {
        (elements[0], elements[1])
    }
}
