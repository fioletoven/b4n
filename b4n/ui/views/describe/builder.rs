use b4n_config::themes::YamlSyntaxColors;
use k8s_openapi::serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::ui::presentation::StyledLine;
use crate::ui::views::describe::utils::{ValueKind, aligned_property, element, header, none, property};

/// Simplifies building Text describe sections.
pub struct TextSectionBuilder<'a> {
    colors: &'a YamlSyntaxColors,
    lines: &'a mut Vec<StyledLine>,
    indent: usize,
    width: Option<usize>,
}

impl<'a> TextSectionBuilder<'a> {
    /// Creates new [`TextSectionBuilder`] instance.
    pub fn new(colors: &'a YamlSyntaxColors, lines: &'a mut Vec<StyledLine>) -> Self {
        Self {
            colors,
            lines,
            indent: 0,
            width: None,
        }
    }

    /// Starts new empty section with specified indentations and properties width.
    pub fn start_empty(&mut self, indent: usize, width: Option<usize>) {
        self.indent = indent;
        self.width = width;
        self.lines.push(StyledLine::default());
    }

    /// Starts new section with specified indentations and properties width.
    pub fn start_section(&mut self, name: &str, header_indent: usize, indent: usize, width: Option<usize>) {
        self.lines.push(StyledLine::default());
        self.sub_section(name, header_indent, indent, width);
    }

    /// Adds subsection with new indentations and properties width.
    pub fn sub_section(&mut self, name: &str, header_indent: usize, indent: usize, width: Option<usize>) {
        self.lines.push(header(self.colors, name, header_indent));
        self.indent = indent;
        self.width = width;
    }

    /// Adds empty line.
    pub fn add_empty(&mut self) {
        self.lines.push(StyledLine::default());
    }

    /// Adds `--none--` line.
    pub fn add_none(&mut self) {
        self.lines.push(none(self.colors));
    }

    /// Adds string key value line.
    pub fn add_str(&mut self, key: &str, value: Option<impl Into<String>>) {
        self.add_line(key, value.map(Into::into).unwrap_or_default(), ValueKind::String);
    }

    /// Adds numeric key value line.
    pub fn add_num(&mut self, key: &str, value: Option<impl Into<String>>) {
        self.add_line(key, value.map(Into::into).unwrap_or_default(), ValueKind::Numeric);
    }

    /// Adds numeric key value line.
    pub fn add_inum(&mut self, key: &str, value: Option<i64>) {
        self.add_line(key, value.map(|v| v.to_string()).unwrap_or_default(), ValueKind::Numeric);
    }

    /// Adds bool key value line.
    pub fn add_bool(&mut self, key: &str, value: Option<bool>) {
        self.add_line(key, value.unwrap_or_default().to_string(), ValueKind::Boolean);
    }

    /// Adds key value line.
    pub fn add_line(&mut self, key: &str, value: impl Into<String>, kind: ValueKind) {
        let line = if let Some(width) = self.width {
            aligned_property(self.colors, key, value, kind, self.indent, width)
        } else {
            property(self.colors, key, value, kind, self.indent)
        };

        self.lines.push(line);
    }

    /// Adds `BTreeMap` as a list.
    pub fn add_btmap(&mut self, title: &str, source: Option<&BTreeMap<String, String>>) {
        self.add_list(title, source.map(|s| s.iter().map(|(k, v)| (k.as_str(), v.as_str()))));
    }

    /// Adds `Map` as a list.
    pub fn add_map(&mut self, title: &str, source: Option<&Map<String, Value>>) {
        self.add_list(
            title,
            source.map(|s| s.iter().map(|(k, v)| (k.as_str(), v.as_str().unwrap_or_default()))),
        );
    }

    fn add_list<'b>(&mut self, title: &str, source: Option<impl Iterator<Item = (&'b str, &'b str)>>) {
        self.lines.push(header(self.colors, title, 0));

        if let Some(source) = source {
            let mut has_elements = false;

            for (key, value) in source {
                if key != "kubectl.kubernetes.io/last-applied-configuration" {
                    self.lines.push(element(self.colors, key, value));
                    has_elements = true;
                }
            }

            if !has_elements {
                self.lines.push(none(self.colors));
            }
        } else {
            self.lines.push(none(self.colors));
        }
    }
}
