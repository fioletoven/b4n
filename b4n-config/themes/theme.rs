use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::themes::{LineColors, TextColors, to_syntect_color};
use crate::{Config, ConfigError, DEFAULT_THEME_NAME, Persistable};

/// Represents header colors.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct HeaderColors {
    pub text: TextColors,
    pub context: TextColors,
    pub namespace: TextColors,
    pub resource: TextColors,
    pub name: TextColors,
    pub count: TextColors,
    pub info: TextColors,
    pub disconnected: TextColors,
    pub previous: TextColors,
}

/// Represents footer colors.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct FooterColors {
    pub text: TextColors,
    pub trail: TextColors,
    pub info: TextColors,
    pub error: TextColors,
}

/// Represents filter colors.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct FilterColors {
    pub input: TextColors,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<TextColors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TextColors>,
}

/// Represents kubernetes resource colors.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ResourceColors {
    pub ready: LineColors,
    pub in_progress: LineColors,
    pub terminating: LineColors,
    pub completed: LineColors,
}

/// Represents colors for UI control.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ControlColors {
    pub normal: TextColors,
    pub focused: TextColors,
}

/// Represents colors for modal dialogs.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ModalColors {
    pub text: TextColors,
    pub selector: SelectColors,
    pub checkbox: ControlColors,
    pub btn_accent: ControlColors,
    pub btn_delete: ControlColors,
    pub btn_cancel: ControlColors,
}

/// Represents colors for selector widget.
#[derive(Serialize, Deserialize, Clone)]
pub struct SelectColors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<TextColors>,
    pub normal: TextColors,
    pub normal_hl: TextColors,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<TextColors>,
    pub filter: FilterColors,
}

impl Default for SelectColors {
    fn default() -> Self {
        Self {
            normal: TextColors::dim(Color::Gray, Color::Yellow, Color::DarkGray),
            normal_hl: TextColors::dim(Color::DarkGray, Color::Blue, Color::Gray),
            header: Some(TextColors::bg(Color::DarkGray, Color::Gray)),
            filter: FilterColors {
                input: TextColors::dim(Color::LightCyan, Color::LightYellow, Color::DarkGray),
                prompt: Some(TextColors::bg(Color::LightBlue, Color::DarkGray)),
                error: Some(TextColors::bg(Color::LightRed, Color::DarkGray)),
            },
            cursor: Some(TextColors::bg(Color::Reset, Color::Gray)),
        }
    }
}

/// Represents colors for syntax highlighting.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SyntaxColors {
    pub yaml: YamlSyntaxColors,
    pub logs: LogsSyntaxColors,
}

/// Represents colors for YAML syntax highlighting.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct YamlSyntaxColors {
    pub normal: TextColors,
    pub property: TextColors,
    pub string: TextColors,
    pub numeric: TextColors,
    pub language: TextColors,
    pub timestamp: TextColors,
    pub search: Color,
    pub select: Color,
}

/// Represents colors for logs syntax highlighting.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct LogsSyntaxColors {
    pub string: TextColors,
    pub error: TextColors,
    pub timestamp: TextColors,
    pub search: Color,
    pub select: Color,
}

/// All colors in theme.
#[derive(Serialize, Deserialize, Clone)]
pub struct ThemeColors {
    pub text: TextColors,
    pub cursor: TextColors,
    pub header: HeaderColors,
    pub footer: FooterColors,
    pub filter: SelectColors,
    pub search: SelectColors,
    pub command_palette: SelectColors,
    pub side_select: SelectColors,
    pub mouse_menu: SelectColors,
    pub modal: ModalColors,
    pub line: ResourceColors,
    pub syntax: SyntaxColors,
}

#[derive(Default, Serialize, Deserialize)]
struct ColorsDefinition {
    #[serde(skip_serializing)]
    pub palette: Option<HashMap<String, String>>,
    pub colors: Value,
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
                text: TextColors::bg(Color::DarkGray, Color::Reset),
                cursor: TextColors::bg(Color::DarkGray, Color::Gray),
                header: HeaderColors {
                    text: TextColors::dim(Color::Gray, Color::LightYellow, Color::DarkGray),
                    context: TextColors::bg(Color::White, Color::Rgb(216, 0, 96)),
                    namespace: TextColors::bg(Color::DarkGray, Color::Rgb(253, 202, 79)),
                    resource: TextColors::bg(Color::DarkGray, Color::Rgb(92, 166, 227)),
                    name: TextColors::bg(Color::DarkGray, Color::Rgb(229, 233, 240)),
                    count: TextColors::bg(Color::DarkGray, Color::Rgb(170, 217, 46)),
                    info: TextColors::bg(Color::White, Color::Rgb(153, 113, 195)),
                    disconnected: TextColors::bg(Color::White, Color::LightRed),
                    previous: TextColors::new(Color::DarkGray),
                },
                footer: FooterColors {
                    text: TextColors::bg(Color::Gray, Color::DarkGray),
                    trail: TextColors::dim(Color::Blue, Color::Yellow, Color::DarkGray),
                    info: TextColors::bg(Color::LightGreen, Color::DarkGray),
                    error: TextColors::bg(Color::LightRed, Color::DarkGray),
                },
                filter: SelectColors::default(),
                search: SelectColors::default(),
                command_palette: SelectColors::default(),
                side_select: SelectColors {
                    normal: TextColors::dim(Color::Gray, Color::Yellow, Color::DarkGray),
                    normal_hl: TextColors::dim(Color::DarkGray, Color::Blue, Color::Gray),
                    filter: FilterColors {
                        input: TextColors::bg(Color::LightBlue, Color::DarkGray),
                        ..Default::default()
                    },
                    header: None,
                    cursor: None,
                },
                mouse_menu: SelectColors::default(),
                modal: ModalColors {
                    text: TextColors::bg(Color::Gray, Color::DarkGray),
                    selector: SelectColors::default(),
                    checkbox: ControlColors {
                        normal: TextColors::bg(Color::Gray, Color::DarkGray),
                        focused: TextColors::bg(Color::LightMagenta, Color::DarkGray),
                    },
                    btn_accent: ControlColors {
                        normal: TextColors::bg(Color::White, Color::DarkGray),
                        focused: TextColors::bg(Color::White, Color::LightBlue),
                    },
                    btn_delete: ControlColors {
                        normal: TextColors::bg(Color::White, Color::DarkGray),
                        focused: TextColors::bg(Color::White, Color::LightRed),
                    },
                    btn_cancel: ControlColors {
                        normal: TextColors::bg(Color::White, Color::DarkGray),
                        focused: TextColors::bg(Color::White, Color::LightGreen),
                    },
                },
                line: ResourceColors {
                    ready: LineColors {
                        normal: TextColors::new(Color::LightBlue),
                        normal_hl: TextColors::bg(Color::DarkGray, Color::LightBlue),
                        selected: TextColors::new(Color::LightGreen),
                        selected_hl: TextColors::bg(Color::DarkGray, Color::LightGreen),
                    },
                    in_progress: LineColors {
                        normal: TextColors::new(Color::Red),
                        normal_hl: TextColors::bg(Color::DarkGray, Color::LightRed),
                        selected: TextColors::new(Color::LightGreen),
                        selected_hl: TextColors::bg(Color::DarkGray, Color::LightGreen),
                    },
                    terminating: LineColors {
                        normal: TextColors::new(Color::Magenta),
                        normal_hl: TextColors::bg(Color::DarkGray, Color::LightMagenta),
                        selected: TextColors::new(Color::LightGreen),
                        selected_hl: TextColors::bg(Color::DarkGray, Color::LightGreen),
                    },
                    completed: LineColors {
                        normal: TextColors::new(Color::Gray),
                        normal_hl: TextColors::bg(Color::Gray, Color::DarkGray),
                        selected: TextColors::new(Color::LightGreen),
                        selected_hl: TextColors::bg(Color::DarkGray, Color::LightGreen),
                    },
                },
                syntax: SyntaxColors {
                    yaml: YamlSyntaxColors {
                        normal: TextColors::new(Color::DarkGray),
                        property: TextColors::new(Color::Green),
                        string: TextColors::new(Color::Gray),
                        numeric: TextColors::new(Color::Blue),
                        language: TextColors::new(Color::LightBlue),
                        timestamp: TextColors::new(Color::Magenta),
                        search: Color::Rgb(135, 114, 72),
                        select: Color::DarkGray,
                    },
                    logs: LogsSyntaxColors {
                        string: TextColors::new(Color::Gray),
                        error: TextColors::new(Color::Red),
                        timestamp: TextColors::new(Color::Magenta),
                        search: Color::Rgb(135, 114, 72),
                        select: Color::DarkGray,
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
                get_theme_item("meta.mapping.key", self.colors.syntax.yaml.property),
                get_theme_item("string -meta.mapping.key, constant.character", self.colors.syntax.yaml.string),
                get_theme_item("punctuation.definition.string", self.colors.syntax.yaml.normal),
                get_theme_item("constant.numeric", self.colors.syntax.yaml.numeric),
                get_theme_item("constant.language", self.colors.syntax.yaml.language),
                get_theme_item("constant.other.timestamp", self.colors.syntax.yaml.timestamp),
            ],
        }
    }
}

impl Persistable<Theme> for Theme {
    /// Returns the default theme file path: `HOME/b4n/themes/default.yaml`.
    fn default_path() -> PathBuf {
        Config::themes_dir().join(format!("{DEFAULT_THEME_NAME}.yaml"))
    }

    async fn load(path: &Path) -> Result<Theme, ConfigError> {
        let mut file = File::open(path).await?;

        let mut theme_str = String::new();
        file.read_to_string(&mut theme_str).await?;

        let mut definitions = serde_yaml::from_str::<ColorsDefinition>(&theme_str)?;
        if let Some(palette) = &definitions.palette
            && !palette.is_empty()
        {
            update_colors(&mut definitions.colors, palette);
            theme_str = serde_yaml::to_string(&definitions)?;
        }

        Ok(serde_yaml::from_str::<Theme>(&theme_str)?)
    }

    async fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let history_str = serde_yaml::to_string(self)?;

        let mut file = File::create(path).await?;
        file.write_all(history_str.as_bytes()).await?;
        file.flush().await?;

        Ok(())
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

fn update_colors(colors: &mut Value, palette: &HashMap<String, String>) {
    let mut stack = vec![colors];

    while let Some(current) = stack.pop() {
        match current {
            Value::Mapping(map) => {
                for v in map.values_mut() {
                    stack.push(v);
                }
            },
            Value::Sequence(sequence) => {
                for v in sequence {
                    stack.push(v);
                }
            },
            Value::String(string) => {
                *string = string
                    .split(':')
                    .map(|c| if palette.contains_key(c) { &palette[c] } else { c })
                    .collect::<Vec<&str>>()
                    .join(":");
            },
            _ => (),
        }
    }
}
