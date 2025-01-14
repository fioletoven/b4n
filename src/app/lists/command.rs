use crate::{kubernetes::resources::Kind, ui::ResponseEvent, utils::truncate};

use super::Row;

/// Command list item.
#[derive(Default)]
pub struct Command {
    pub uid: Option<String>,
    pub group: String,
    pub name: String,
    pub response: ResponseEvent,
    description: Option<String>,
    icon: Option<String>,
    aliases: Option<Vec<String>>,
}

impl Command {
    /// Creates new [`Command`] instance.
    pub fn new(name: &str) -> Self {
        Self {
            uid: Some(format!("_command:{}_", name)),
            group: "command".to_owned(),
            name: name.to_owned(),
            icon: Some("".to_owned()),
            ..Default::default()
        }
    }

    /// Creates new [`Command`] instance from [`Kind`].
    pub fn from(kind: &Kind) -> Self {
        Self {
            uid: kind.uid().map(String::from),
            group: "resource".to_owned(),
            name: kind.name().to_owned(),
            response: ResponseEvent::ChangeKind(kind.name().to_owned()),
            ..Default::default()
        }
    }

    /// Sets the provided description.
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_owned());
        self
    }

    /// Sets the provided aliases.
    pub fn with_aliases(mut self, aliases: &[&str]) -> Self {
        self.aliases = Some(aliases.iter().map(|a| (*a).to_owned()).collect());
        self
    }

    /// Sets the provided response.
    pub fn with_response(mut self, response: ResponseEvent) -> Self {
        self.response = response;
        self
    }

    fn get_text_width(&self, width: usize) -> usize {
        self.icon
            .as_ref()
            .map(|i| width.max(i.chars().count() + 1) - i.chars().count() - 1)
            .unwrap_or(width)
    }

    fn add_icon(&self, text: String) -> String {
        if let Some(icon) = &self.icon {
            format!("{} {}", text, icon)
        } else {
            text
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
        let text_width = self.get_text_width(width);
        if let Some(descr) = &self.description {
            if self.name.chars().count() + descr.chars().count() + 1 > text_width {
                let text = format!("{} [{}", self.name, descr);
                self.add_icon(format!("{}]", truncate(&text, text_width + 1)))
            } else {
                let remaining_len = text_width - descr.chars().count() - 1;
                self.add_icon(format!("{1:<0$} [{2}]", remaining_len, self.name, descr))
            }
        } else {
            self.add_icon(format!("{1:<0$}", text_width, self.name))
        }
    }

    fn column_text(&self, column: usize) -> &str {
        match column {
            0 => &self.group,
            1 => &self.name,
            _ => "n/a",
        }
    }

    /// Returns `true` if the given `pattern` is found in the command name or its aliases.
    fn contains(&self, pattern: &str) -> bool {
        if self.name.contains(pattern) {
            return true;
        }

        if let Some(aliases) = &self.aliases {
            return aliases.iter().any(|a| a.contains(pattern));
        }

        false
    }

    /// Returns `true` if the command name or its aliases starts with the given `pattern`.
    fn starts_with(&self, pattern: &str) -> bool {
        if self.name.starts_with(pattern) {
            return true;
        }

        if let Some(aliases) = &self.aliases {
            return aliases.iter().any(|a| a.starts_with(pattern));
        }

        false
    }

    /// Returns `true` if the given `pattern` is equal to the command name or its aliases.
    fn is_equal(&self, pattern: &str) -> bool {
        if self.name == pattern {
            return true;
        }

        if let Some(aliases) = &self.aliases {
            return aliases.iter().any(|a| a == pattern);
        }

        false
    }
}
