use b4n_config::themes::YamlSyntaxColors;

use crate::ui::presentation::StyledLine;
use crate::ui::views::describe::utils::{ValueKind, aligned_property, header, property};

pub struct TextSectionBuilder<'a> {
    colors: &'a YamlSyntaxColors,
    lines: &'a mut Vec<StyledLine>,
    indent: usize,
    width: Option<usize>,
}

impl<'a> TextSectionBuilder<'a> {
    pub fn new(colors: &'a YamlSyntaxColors, lines: &'a mut Vec<StyledLine>) -> Self {
        Self {
            colors,
            lines,
            indent: 0,
            width: None,
        }
    }

    pub fn start_empty(&mut self, indent: usize, width: Option<usize>) {
        self.indent = indent;
        self.width = width;
        self.lines.push(StyledLine::default());
    }

    pub fn start_section(&mut self, name: &str, indent: usize, width: Option<usize>) {
        self.indent = indent;
        self.width = width;
        self.lines.push(StyledLine::default());
        self.lines.push(header(self.colors, name));
    }

    pub fn add_str(&mut self, name: &str, value: Option<&str>) {
        self.add_line(name, value.unwrap_or_default(), ValueKind::String);
    }

    pub fn add_num(&mut self, name: &str, value: Option<&str>) {
        self.add_line(name, value.unwrap_or_default(), ValueKind::Numeric);
    }

    pub fn add_bool(&mut self, name: &str, value: Option<bool>) {
        self.add_line(name, value.unwrap_or_default().to_string(), ValueKind::Boolean);
    }

    fn add_line(&mut self, name: &str, value: impl Into<String>, kind: ValueKind) {
        let line = if let Some(width) = self.width {
            aligned_property(self.colors, name, value, kind, self.indent, width)
        } else {
            property(self.colors, name, value, kind, self.indent)
        };

        self.lines.push(line);
    }
}
