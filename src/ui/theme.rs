use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::colors::{to_syntect_color, LineColors, TextColors};

/// Represents kubernetes resource colors.
#[derive(Default, Serialize, Deserialize, Copy, Clone)]
pub struct ResourceColors {
    pub ready: LineColors,
    pub in_progress: LineColors,
    pub terminating: LineColors,
    pub completed: LineColors,
}

/// Represents colors for button.
#[derive(Default, Serialize, Deserialize, Copy, Clone)]
pub struct ButtonColors {
    pub normal: TextColors,
    pub focused: TextColors,
}

/// Represents colors for modal dialogs.
#[derive(Default, Serialize, Deserialize, Copy, Clone)]
pub struct ModalColors {
    pub colors: TextColors,
    pub btn_delete: ButtonColors,
    pub btn_cancel: ButtonColors,
}

/// Represents colors for selector widget.
#[derive(Default, Serialize, Deserialize, Copy, Clone)]
pub struct SelectColors {
    pub normal: TextColors,
    pub normal_hl: TextColors,
    pub filter: TextColors,
    pub prompt: TextColors,
}

/// Represents colors for syntax highlighting.
#[derive(Default, Serialize, Deserialize, Copy, Clone)]
pub struct SyntaxColors {
    pub yaml: YamlSyntaxColors,
}

/// Represents colors for YAML syntax highlighting.
#[derive(Default, Serialize, Deserialize, Copy, Clone)]
pub struct YamlSyntaxColors {
    pub normal: TextColors,
    pub property: TextColors,
    pub string: TextColors,
    pub numeric: TextColors,
    pub language: TextColors,
    pub timestamp: TextColors,
}

/// All colors in theme.
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct ThemeColors {
    pub context: TextColors,
    pub namespace: TextColors,
    pub resource: TextColors,
    pub count: TextColors,
    pub info: TextColors,
    pub disconnected: TextColors,
    pub header: TextColors,
    pub modal: ModalColors,
    pub command_palette: SelectColors,
    pub side_select: SelectColors,
    pub line: ResourceColors,
    pub syntax: SyntaxColors,
}

/// Theme used in the application.
#[derive(Serialize, Deserialize, Clone)]
pub struct Theme {
    pub colors: ThemeColors,
}

impl Default for Theme {
    /// Returns TUI default theme for the application.
    fn default() -> Self {
        Theme {
            colors: ThemeColors {
                context: TextColors::new(Color::White, Color::Rgb(216, 0, 96)),
                namespace: TextColors::new(Color::DarkGray, Color::Rgb(253, 202, 79)),
                resource: TextColors::new(Color::DarkGray, Color::Rgb(92, 166, 227)),
                count: TextColors::new(Color::DarkGray, Color::Rgb(170, 217, 46)),
                info: TextColors::new(Color::White, Color::Rgb(153, 113, 195)),
                disconnected: TextColors::new(Color::White, Color::LightRed),
                header: TextColors::new(Color::Gray, Color::DarkGray),
                modal: ModalColors {
                    colors: TextColors::new(Color::Gray, Color::DarkGray),
                    btn_delete: ButtonColors {
                        normal: TextColors::new(Color::White, Color::DarkGray),
                        focused: TextColors::new(Color::White, Color::LightRed),
                    },
                    btn_cancel: ButtonColors {
                        normal: TextColors::new(Color::White, Color::DarkGray),
                        focused: TextColors::new(Color::White, Color::LightGreen),
                    },
                },
                command_palette: SelectColors {
                    normal: TextColors::new(Color::Gray, Color::DarkGray),
                    normal_hl: TextColors::new(Color::DarkGray, Color::Gray),
                    filter: TextColors::new(Color::Blue, Color::DarkGray),
                    prompt: TextColors::new(Color::Blue, Color::DarkGray),
                },
                side_select: SelectColors {
                    normal: TextColors::new(Color::Gray, Color::DarkGray),
                    normal_hl: TextColors::new(Color::DarkGray, Color::Gray),
                    filter: TextColors::new(Color::Blue, Color::DarkGray),
                    prompt: TextColors::new(Color::Blue, Color::DarkGray),
                },
                line: ResourceColors {
                    ready: LineColors {
                        normal: TextColors::new(Color::LightBlue, Color::Reset),
                        normal_hl: TextColors::new(Color::DarkGray, Color::LightBlue),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                    in_progress: LineColors {
                        normal: TextColors::new(Color::Red, Color::Reset),
                        normal_hl: TextColors::new(Color::DarkGray, Color::LightRed),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                    terminating: LineColors {
                        normal: TextColors::new(Color::Magenta, Color::Reset),
                        normal_hl: TextColors::new(Color::DarkGray, Color::LightMagenta),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                    completed: LineColors {
                        normal: TextColors::new(Color::Gray, Color::Reset),
                        normal_hl: TextColors::new(Color::Gray, Color::DarkGray),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                },
                syntax: SyntaxColors {
                    yaml: YamlSyntaxColors {
                        normal: TextColors::new(Color::Gray, Color::Reset),
                        property: TextColors::new(Color::Green, Color::Reset),
                        string: TextColors::new(Color::Black, Color::Reset),
                        numeric: TextColors::new(Color::Blue, Color::Reset),
                        language: TextColors::new(Color::LightBlue, Color::Reset),
                        timestamp: TextColors::new(Color::Magenta, Color::Reset),
                    },
                },
            },
        }
    }
}

impl Theme {
    /// Returns the syntect theme for highlighting YAML syntax.
    pub fn build_syntect_yaml_theme(&self) -> syntect::highlighting::Theme {
        syntect::highlighting::Theme {
            name: None,
            author: None,
            settings: syntect::highlighting::ThemeSettings {
                foreground: Some(to_syntect_color(self.colors.syntax.yaml.normal.fg)),
                background: Some(to_syntect_color(self.colors.syntax.yaml.normal.bg)),
                ..Default::default()
            },
            scopes: vec![
                get_theme_item("entity.name", self.colors.syntax.yaml.property),
                get_theme_item("string.quoted, string.unquoted", self.colors.syntax.yaml.string),
                get_theme_item("constant.numeric", self.colors.syntax.yaml.numeric),
                get_theme_item("constant.language", self.colors.syntax.yaml.language),
                get_theme_item("constant.other.timestamp", self.colors.syntax.yaml.timestamp),
            ],
        }
    }
}

fn get_theme_item(scope: &str, colors: TextColors) -> syntect::highlighting::ThemeItem {
    syntect::highlighting::ThemeItem {
        scope: scope.parse().unwrap(),
        style: syntect::highlighting::StyleModifier {
            foreground: Some(to_syntect_color(colors.fg)),
            background: Some(to_syntect_color(colors.bg)),
            font_style: None,
        },
    }
}
