use b4n_common::truncate;
use b4n_kube::Port;
use b4n_list::{BasicFilterContext, Filterable, Row};
use std::{borrow::Cow, path::PathBuf};

use crate::ResponseEvent;

#[cfg(test)]
#[path = "./action.tests.rs"]
mod action_tests;

/// Command palette action.
#[derive(Default, Clone)]
pub struct ActionItem {
    pub uid: String,
    pub group: String,
    pub name: String,
    pub response: ResponseEvent,
    pub id: Option<usize>,
    description: Option<String>,
    icon: Option<&'static str>,
    aliases: Option<Vec<String>>,
}

impl ActionItem {
    /// Creates new [`ActionItem`] instance.
    pub fn new(name: &str) -> Self {
        Self {
            uid: format!("_action:{name}_"),
            group: "action".to_owned(),
            name: name.to_owned(),
            icon: Some(""),
            ..Default::default()
        }
    }

    /// Creates new raw [`ActionItem`] instance.
    pub fn raw(uid: String, group: String, name: String, icon: Option<&'static str>) -> Self {
        Self {
            uid,
            group,
            name,
            icon,
            ..Default::default()
        }
    }

    /// Creates new [`ActionItem`] instance with action response.
    pub fn action(name: &str, action: &'static str) -> Self {
        ActionItem::new(name).with_response(ResponseEvent::Action(action))
    }

    /// Creates new [`ActionItem`] instance for mouse menu.
    pub fn menu(id: usize, name: &str, action: &'static str) -> Self {
        ActionItem::new(name)
            .with_response(ResponseEvent::Action(action))
            .with_id(id)
            .with_no_icon()
    }

    /// Creates new [`ActionItem`] instance `command palette` for mouse menu.
    pub fn command_palette() -> Self {
        ActionItem::new(" command palette")
            .with_response(ResponseEvent::Action("palette"))
            .with_id(50)
            .with_no_icon()
    }

    /// Creates new [`ActionItem`] instance `back` for mouse menu.
    pub fn back() -> Self {
        ActionItem::new("󰕍 back")
            .with_response(ResponseEvent::Cancelled)
            .with_id(100)
            .with_no_icon()
    }

    /// Creates new [`ActionItem`] instance from [`PathBuf`].
    pub fn from_path(path: PathBuf) -> Self {
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        Self {
            uid: path.as_os_str().to_string_lossy().to_string(),
            group: "path".to_owned(),
            name: name.clone(),
            response: ResponseEvent::ChangeTheme(name),
            ..Default::default()
        }
    }

    /// Creates new [`ActionItem`] instance from [`Port`].
    pub fn from_port(port: &Port) -> Self {
        Self {
            uid: format!("_{}:{}:{}_", port.port, port.name, port.protocol),
            group: "resource port".to_owned(),
            name: port.port.to_string(),
            description: Some(format!("{} ({})", port.name, port.protocol)),
            aliases: Some(vec![port.name.clone()]),
            ..Default::default()
        }
    }

    /// Hides icon for this action instance.
    pub fn with_no_icon(mut self) -> Self {
        self.icon = None;
        self
    }

    /// Sets sort `id` for this action instance.
    pub fn with_id(mut self, id: usize) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the provided description.
    pub fn with_description(mut self, description: &str) -> Self {
        if !description.is_empty() {
            self.description = Some(description.to_owned());
        }

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

    /// Adds specified aliases to the existing ones.
    pub fn add_aliases(&mut self, mut aliases: Vec<String>) {
        if let Some(existing) = &mut self.aliases {
            existing.append(&mut aliases);
        } else {
            self.aliases = Some(aliases);
        }
    }

    fn get_text_width(&self, width: usize) -> usize {
        self.icon.as_ref().map_or(width, |i| {
            let icon_width = i.chars().count();
            width.saturating_sub(icon_width + 1)
        })
    }

    fn get_name_width(&self) -> usize {
        self.name.chars().filter(|c| *c != '[' && *c != ']').count()
    }

    fn add_icon(&self, text: &mut String) {
        if let Some(icon) = &self.icon {
            text.push(' ');
            text.push_str(icon);
        }
    }
}

impl Row for ActionItem {
    fn uid(&self) -> &str {
        &self.uid
    }

    fn group(&self) -> &str {
        &self.group
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, width: usize) -> String {
        let text_width = self.get_text_width(width);
        let name_width = self.get_name_width().min(text_width);

        let mut text = String::with_capacity(text_width + 2);
        text.push_str(truncate(&self.name, text_width));

        if let Some(descr) = &self.description {
            let descr_width = descr.chars().count();
            if name_width + descr_width + 1 > text_width {
                if name_width + 2 < text_width {
                    text.push_str(" [");
                    text.push_str(truncate(descr, text_width - text.len() + 1));
                    text.push(']');
                }
            } else {
                let padding_len = text_width.saturating_sub(name_width).saturating_sub(descr_width);
                (0..padding_len).for_each(|_| text.push(' '));
                text.push('[');
                text.push_str(descr);
                text.push(']');
            }
        } else {
            let padding_len = text_width.saturating_sub(name_width);
            (0..padding_len).for_each(|_| text.push(' '));
        }

        self.add_icon(&mut text);
        text
    }

    fn column_text(&self, column: usize) -> Cow<'_, str> {
        Cow::Borrowed(match column {
            0 => &self.group,
            1 => &self.name,
            _ => "n/a",
        })
    }

    fn column_sort_text(&self, column: usize) -> &str {
        match column {
            0 => &self.group,
            1 => &self.name,
            _ => "n/a",
        }
    }

    /// Returns `true` if the given `pattern` is found in the action name or its aliases.
    fn contains(&self, pattern: &str) -> bool {
        if self.name.contains(pattern) {
            return true;
        }

        if let Some(aliases) = &self.aliases {
            return aliases.iter().any(|a| a.contains(pattern));
        }

        false
    }

    /// Returns `true` if the action name or its aliases starts with the given `pattern`.
    fn starts_with(&self, pattern: &str) -> bool {
        if self.name.starts_with(pattern) {
            return true;
        }

        if let Some(aliases) = &self.aliases {
            return aliases.iter().any(|a| a.starts_with(pattern));
        }

        false
    }

    /// Returns `true` if the given `pattern` is equal to the action name or its aliases.
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

impl Filterable<BasicFilterContext> for ActionItem {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.contains(&context.pattern)
    }
}
