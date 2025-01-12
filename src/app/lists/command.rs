use crate::utils::truncate;

use super::Row;

pub enum CommandType {
    ChangeKind(String),
    ChangeContext,
    ExitApplication,
}

/// UI command.
pub struct Command {
    pub uid: Option<String>,
    pub group: String,
    pub name: String,
    aliases: Option<Vec<String>>,
    command: CommandType,
}

impl Command {
    /// Creates new [`Command`] instance.
    pub fn new(group: String, name: String, aliases: Option<Vec<String>>) -> Self {
        Self {
            uid: Some(format!("_{}:{}_", group, name)),
            group,
            name,
            aliases,
            command: CommandType::ChangeContext,
        }
    }
}

impl Row for Command {
    fn uid(&self) -> Option<&str> {
        self.uid.as_deref()
    }

    fn group(&self) -> &str {
        &self.group
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, width: usize) -> String {
        format!("{1:<0$}", width, truncate(self.name.as_str(), width))
    }

    fn column_text(&self, column: usize) -> &str {
        match column {
            0 => &self.group,
            1 => &self.name,
            _ => "n/a",
        }
    }
}
