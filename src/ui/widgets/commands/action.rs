use kube::config::NamedContext;

use crate::{
    kubernetes::{kinds::KindItem, resources::Port},
    ui::{
        ResponseEvent,
        lists::{BasicFilterContext, Filterable, Row},
    },
    utils::truncate,
};

/// Command palette action.
#[derive(Default)]
pub struct ActionItem {
    pub uid: Option<String>,
    pub group: String,
    pub name: String,
    pub response: ResponseEvent,
    description: Option<String>,
    icon: Option<String>,
    aliases: Option<Vec<String>>,
}

impl ActionItem {
    /// Creates new [`ActionItem`] instance.
    pub fn new(name: &str) -> Self {
        Self {
            uid: Some(format!("_action:{name}_")),
            group: "action".to_owned(),
            name: name.to_owned(),
            icon: Some("".to_owned()),
            ..Default::default()
        }
    }

    /// Creates new [`ActionItem`] instance from [`KindItem`].
    pub fn from_kind(kind: &KindItem) -> Self {
        Self {
            uid: kind.uid().map(String::from),
            group: "resource".to_owned(),
            name: kind.name().to_owned(),
            response: ResponseEvent::ChangeKind(kind.name().to_owned()),
            ..Default::default()
        }
    }

    /// Creates new [`ActionItem`] instance from [`NamedContext`].
    pub fn from_context(context: &NamedContext) -> Self {
        Self {
            uid: Some(format!(
                "_{}:{}_",
                context.name,
                context.context.as_ref().map(|c| c.cluster.as_str()).unwrap_or_default()
            )),
            group: "context".to_owned(),
            name: context.name.clone(),
            response: ResponseEvent::ChangeContext(context.name.clone()),
            description: context.context.as_ref().map(|c| c.cluster.clone()),
            ..Default::default()
        }
    }

    /// Creates new [`ActionItem`] instance from [`Port`].
    pub fn from_port(port: &Port) -> Self {
        Self {
            uid: Some(format!("_{}:{}:{}_", port.port, port.name, port.protocol)),
            group: "resource port".to_owned(),
            name: port.port.to_string(),
            description: Some(format!("{} ({})", port.name, port.protocol)),
            aliases: Some(vec![port.name.clone()]),
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
            .map_or(width, |i| width.max(i.chars().count() + 1) - i.chars().count() - 1)
    }

    fn add_icon(&self, text: &mut String) {
        if let Some(icon) = &self.icon {
            text.push(' ');
            text.push_str(icon);
        }
    }
}

impl Row for ActionItem {
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
        let name_width = self.name.chars().count();

        let mut text = String::with_capacity(text_width + 2);
        text.push_str(&self.name);

        if let Some(descr) = &self.description {
            let descr_width = descr.chars().count();
            if name_width + descr_width + 1 > text_width {
                text.push_str(" [");
                text.push_str(truncate(descr, text_width - text.len() + 1));
                text.push(']');
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

    fn column_text(&self, column: usize) -> &str {
        match column {
            0 => &self.group,
            1 => &self.name,
            _ => "n/a",
        }
    }

    fn column_sort_text(&self, column: usize) -> &str {
        self.column_text(column)
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
