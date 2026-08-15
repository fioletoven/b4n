use ratatui_core::style::Color;
use serde::de::{self, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::themes::colors::SelectableLineColors;
use crate::themes::{LineColors, TextColors, to_syntect_color};
use crate::{Config, ConfigError, DEFAULT_THEME_NAME, Persistable};

const LIGHT_TEXT: Color = Color::Rgb(164, 164, 184);

/// Represents header colors.
#[derive(Serialize, Deserialize, Clone)]
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

impl Default for HeaderColors {
    fn default() -> Self {
        Self {
            text: TextColors::dim(Color::Gray, Color::LightYellow, Color::DarkGray),
            context: TextColors::bg(Color::White, Color::Rgb(216, 0, 96)),
            namespace: TextColors::bg(Color::DarkGray, Color::Rgb(253, 202, 79)),
            resource: TextColors::bg(Color::DarkGray, Color::Rgb(92, 166, 227)),
            name: TextColors::bg(Color::DarkGray, Color::Rgb(229, 233, 240)),
            count: TextColors::bg(Color::DarkGray, Color::Rgb(170, 217, 46)),
            info: TextColors::bg(Color::White, Color::Rgb(153, 113, 195)),
            disconnected: TextColors::bg(Color::White, Color::LightRed),
            previous: TextColors::new(Color::DarkGray),
        }
    }
}

/// Represents footer colors.
#[derive(Serialize, Deserialize, Clone)]
pub struct FooterColors {
    pub text: TextColors,
    pub trail: TextColors,
    pub info: TextColors,
    pub error: TextColors,
    pub hint: TextColors,
    pub details: FooterDetailsColors,
}

impl Default for FooterColors {
    fn default() -> Self {
        Self {
            text: TextColors::dim(Color::Gray, LIGHT_TEXT, Color::DarkGray),
            trail: TextColors::dim(Color::Blue, Color::Yellow, Color::DarkGray),
            info: TextColors::bg(Color::LightGreen, Color::DarkGray),
            error: TextColors::bg(Color::LightRed, Color::DarkGray),
            hint: TextColors::dim(Color::Black, Color::Yellow, Color::DarkGray),
            details: FooterDetailsColors::default(),
        }
    }
}

/// Represents footer details view colors.
#[derive(Serialize, Deserialize, Clone)]
pub struct FooterDetailsColors {
    pub text: TextColors,
    pub hint: TextColors,
    pub info: LineColors,
    pub error: LineColors,
}

impl Default for FooterDetailsColors {
    fn default() -> Self {
        Self {
            text: TextColors::dim(Color::Gray, LIGHT_TEXT, Color::DarkGray),
            hint: TextColors::bg(Color::DarkGray, Color::Gray),
            info: LineColors {
                normal: TextColors::bg(Color::LightGreen, Color::DarkGray),
                accent: TextColors::bg(Color::Black, Color::LightGreen),
            },
            error: LineColors {
                normal: TextColors::bg(Color::LightRed, Color::DarkGray),
                accent: TextColors::bg(Color::Black, Color::LightRed),
            },
        }
    }
}

/// Represents filter colors.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct FilterColors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<TextColors>,
    pub input: TextColors,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<TextColors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TextColors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<TextColors>,
}

/// Represents list colors.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ListColors {
    pub header: ListHeaderColors,
    pub line: ResourceColors,
    pub line_cached: ResourceColors,
}

/// Represents list header colors.
#[derive(Serialize, Deserialize, Clone)]
pub struct ListHeaderColors {
    pub focused: TextColors,
    pub dimmed: TextColors,
}

impl Default for ListHeaderColors {
    fn default() -> Self {
        Self {
            focused: TextColors::dim(Color::Gray, Color::LightYellow, Color::DarkGray),
            dimmed: TextColors::dim(Color::Gray, LIGHT_TEXT, Color::DarkGray),
        }
    }
}

/// Represents kubernetes resource colors.
#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceColors {
    pub ready: SelectableLineColors,
    pub in_progress: SelectableLineColors,
    pub terminating: SelectableLineColors,
    pub completed: SelectableLineColors,
    pub dimmed: SelectableLineColors,
}

impl Default for ResourceColors {
    fn default() -> Self {
        Self {
            ready: SelectableLineColors {
                normal: TextColors::new(Color::LightBlue),
                accent: TextColors::bg(Color::Black, Color::LightBlue),
                selected: Some(LineColors {
                    normal: TextColors::new(Color::LightGreen),
                    accent: TextColors::bg(Color::Black, Color::LightGreen),
                }),
            },
            in_progress: SelectableLineColors {
                normal: TextColors::new(Color::Red),
                accent: TextColors::bg(Color::Black, Color::LightRed),
                selected: Some(LineColors {
                    normal: TextColors::new(Color::LightGreen),
                    accent: TextColors::bg(Color::Black, Color::LightGreen),
                }),
            },
            terminating: SelectableLineColors {
                normal: TextColors::new(Color::Magenta),
                accent: TextColors::bg(Color::Black, Color::LightMagenta),
                selected: Some(LineColors {
                    normal: TextColors::new(Color::LightGreen),
                    accent: TextColors::bg(Color::Black, Color::LightGreen),
                }),
            },
            completed: SelectableLineColors {
                normal: TextColors::new(Color::Gray),
                accent: TextColors::bg(Color::Gray, Color::Black),
                selected: Some(LineColors {
                    normal: TextColors::new(Color::LightGreen),
                    accent: TextColors::bg(Color::Black, Color::LightGreen),
                }),
            },
            dimmed: SelectableLineColors {
                normal: TextColors::new(Color::Gray),
                accent: TextColors::new(Color::Gray),
                selected: None,
            },
        }
    }
}

impl ResourceColors {
    pub fn cached() -> Self {
        let mut colors = ResourceColors::default();
        colors.ready.normal = TextColors::new(Color::Gray);
        colors.ready.selected = None;
        colors.in_progress.normal = TextColors::new(Color::Gray);
        colors.in_progress.selected = None;
        colors.terminating.normal = TextColors::new(Color::Gray);
        colors.terminating.selected = None;
        colors.completed.normal = TextColors::new(Color::Gray);
        colors.completed.selected = None;
        colors.dimmed.normal = TextColors::new(Color::Gray);
        colors.dimmed.selected = None;
        colors
    }
}

/// Represents colors for UI control.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ControlColors {
    pub normal: TextColors,
    pub hovered: TextColors,
    pub focused: TextColors,
}

impl ControlColors {
    /// Returns [`TextColors`] for normal, hovered or focused control.
    pub fn get(&mut self, is_hovered: bool, is_focused: bool) -> &TextColors {
        if is_hovered {
            &self.hovered
        } else if is_focused {
            &self.focused
        } else {
            &self.normal
        }
    }
}

/// Represents colors for textbox inside modal dialogs.
#[derive(Serialize, Deserialize, Clone)]
pub struct TextBoxModalColors {
    pub caption: ControlColors,
    pub input: ControlColors,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<TextColors>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TextColors>,
}

impl Default for TextBoxModalColors {
    fn default() -> Self {
        Self {
            caption: ControlColors {
                normal: TextColors::bg(Color::Gray, Color::DarkGray),
                hovered: TextColors::bg(Color::LightCyan, Color::DarkGray),
                focused: TextColors::bg(Color::LightMagenta, Color::DarkGray),
            },
            input: ControlColors {
                normal: TextColors::dim(Color::Gray, Color::LightYellow, Color::DarkGray),
                hovered: TextColors::dim(Color::LightCyan, Color::LightYellow, Color::DarkGray),
                focused: TextColors::dim(Color::LightCyan, Color::LightYellow, Color::DarkGray),
            },
            cursor: Some(TextColors::bg(Color::Reset, Color::Gray)),
            error: Some(TextColors::bg(Color::LightRed, Color::DarkGray)),
        }
    }
}

/// Represents colors for select inside modal dialogs.
#[derive(Serialize, Deserialize, Clone)]
pub struct SelectModalColors {
    pub caption: ControlColors,
    pub items: LineColors,
}

impl Default for SelectModalColors {
    fn default() -> Self {
        Self {
            caption: ControlColors {
                normal: TextColors::bg(Color::Gray, Color::DarkGray),
                hovered: TextColors::bg(Color::LightCyan, Color::DarkGray),
                focused: TextColors::bg(Color::LightMagenta, Color::DarkGray),
            },
            items: LineColors {
                normal: TextColors::dim(Color::Gray, Color::Yellow, Color::DarkGray),
                accent: TextColors::dim(Color::DarkGray, Color::Blue, Color::Gray),
            },
        }
    }
}

/// Represents colors for modal dialogs.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModalColors {
    pub text: TextColors,
    pub textbox: TextBoxModalColors,
    pub checkbox: ControlColors,
    pub selector: SelectModalColors,
    pub btn_accent: ControlColors,
    pub btn_delete: ControlColors,
    pub btn_cancel: ControlColors,
}

impl Default for ModalColors {
    fn default() -> Self {
        Self {
            text: TextColors::bg(Color::Gray, Color::DarkGray),
            textbox: TextBoxModalColors::default(),
            checkbox: ControlColors {
                normal: TextColors::bg(Color::Gray, Color::DarkGray),
                hovered: TextColors::bg(Color::LightCyan, Color::DarkGray),
                focused: TextColors::bg(Color::LightMagenta, Color::DarkGray),
            },
            selector: SelectModalColors::default(),
            btn_accent: ControlColors {
                normal: TextColors::bg(Color::White, Color::DarkGray),
                hovered: TextColors::bg(Color::White, Color::Blue),
                focused: TextColors::bg(Color::White, Color::LightBlue),
            },
            btn_delete: ControlColors {
                normal: TextColors::bg(Color::White, Color::DarkGray),
                hovered: TextColors::bg(Color::White, Color::Red),
                focused: TextColors::bg(Color::White, Color::LightRed),
            },
            btn_cancel: ControlColors {
                normal: TextColors::bg(Color::White, Color::DarkGray),
                hovered: TextColors::bg(Color::White, Color::Green),
                focused: TextColors::bg(Color::White, Color::LightGreen),
            },
        }
    }
}

/// Represents colors for selector widget.
#[derive(Serialize, Deserialize, Clone)]
pub struct SelectColors {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<TextColors>,
    pub filter: FilterColors,
    pub items: LineColors,
}

impl Default for SelectColors {
    fn default() -> Self {
        Self {
            header: Some(TextColors::bg(Color::DarkGray, Color::Gray)),
            filter: FilterColors {
                input: TextColors::dim(Color::LightCyan, Color::LightYellow, Color::DarkGray),
                cursor: Some(TextColors::bg(Color::Reset, Color::Gray)),
                prompt: Some(TextColors::bg(Color::LightBlue, Color::DarkGray)),
                error: Some(TextColors::bg(Color::LightRed, Color::DarkGray)),
                button: Some(TextColors::bg(Color::LightBlue, Color::DarkGray)),
            },
            items: LineColors {
                normal: TextColors::dim(Color::Gray, Color::Yellow, Color::DarkGray),
                accent: TextColors::dim(Color::DarkGray, Color::Blue, Color::Gray),
            },
        }
    }
}

/// Represents colors for shell.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ShellColors {
    pub select: Color,
}

/// Represents colors for syntax highlighting.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SyntaxColors {
    pub describe: YamlSyntaxColors,
    pub yaml: YamlSyntaxColors,
    pub logs: LogsSyntaxColors,
}

/// Represents colors for YAML syntax highlighting.
#[derive(Serialize, Deserialize, Clone)]
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

impl Default for YamlSyntaxColors {
    fn default() -> Self {
        Self {
            normal: TextColors::new(Color::DarkGray),
            property: TextColors::new(Color::Green),
            string: TextColors::new(Color::Gray),
            numeric: TextColors::new(Color::Blue),
            language: TextColors::new(Color::LightBlue),
            timestamp: TextColors::new(Color::Magenta),
            search: Color::Rgb(135, 114, 72),
            select: Color::DarkGray,
        }
    }
}

/// Represents colors for logs syntax highlighting.
#[derive(Serialize, Deserialize, Clone)]
pub struct LogsSyntaxColors {
    pub string: TextColors,
    pub info: TextColors,
    pub error: TextColors,
    pub timestamp: TextColors,
    pub search: Color,
    pub select: Color,
    pub containers: Vec<TextColors>,
}

impl Default for LogsSyntaxColors {
    fn default() -> Self {
        Self {
            string: TextColors::new(Color::Gray),
            info: TextColors::new(Color::DarkGray),
            error: TextColors::new(Color::Red),
            timestamp: TextColors::new(Color::Magenta),
            search: Color::Rgb(135, 114, 72),
            select: Color::DarkGray,
            containers: vec![
                TextColors::new(Color::Green),
                TextColors::new(Color::Blue),
                TextColors::new(Color::Cyan),
                TextColors::new(Color::Yellow),
            ],
        }
    }
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
    pub list: ListColors,
    pub shell: ShellColors,
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
                text: TextColors::bg(Color::DarkGray, Color::Reset),
                cursor: TextColors::bg(Color::DarkGray, Color::Gray),
                header: HeaderColors::default(),
                footer: FooterColors::default(),
                filter: SelectColors::default(),
                search: SelectColors::default(),
                command_palette: SelectColors::default(),
                side_select: SelectColors {
                    header: None,
                    filter: FilterColors {
                        input: TextColors::bg(Color::LightBlue, Color::DarkGray),
                        ..Default::default()
                    },
                    items: LineColors {
                        normal: TextColors::dim(Color::Gray, Color::Yellow, Color::DarkGray),
                        accent: TextColors::dim(Color::DarkGray, Color::Blue, Color::Gray),
                    },
                },
                mouse_menu: SelectColors::default(),
                modal: ModalColors::default(),
                list: ListColors {
                    header: ListHeaderColors::default(),
                    line: ResourceColors::default(),
                    line_cached: ResourceColors::cached(),
                },
                shell: ShellColors { select: Color::DarkGray },
                syntax: SyntaxColors {
                    describe: YamlSyntaxColors::default(),
                    yaml: YamlSyntaxColors::default(),
                    logs: LogsSyntaxColors::default(),
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
    /// Returns the default theme file path: `$HOME/.b4n/themes/default.yaml`.
    fn default_path() -> PathBuf {
        Config::themes_dir().join(format!("{DEFAULT_THEME_NAME}.yaml"))
    }

    async fn load(path: &Path) -> Result<Theme, ConfigError> {
        let mut file = File::open(path).await?;

        let mut theme_str = String::new();
        file.read_to_string(&mut theme_str).await?;

        let mut definitions = serde_saphyr::from_str::<ColorsDefinition>(&theme_str)?;
        if let Some(palette) = &definitions.palette
            && !palette.is_empty()
        {
            update_colors(&mut definitions.colors, palette);
            theme_str = serde_saphyr::to_string(&definitions)?;
        }

        Ok(serde_saphyr::from_str::<Theme>(&theme_str)?)
    }

    async fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let history_str = serde_saphyr::to_string(self)?;

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

#[derive(Default, Serialize, Deserialize)]
struct ColorsDefinition {
    #[serde(skip_serializing)]
    pub palette: Option<BTreeMap<String, String>>,
    pub colors: BTreeMap<String, ColorValue>,
}

#[derive(Debug)]
enum ColorValue {
    String(String),
    Mapping(BTreeMap<String, ColorValue>),
    Sequence(Vec<String>),
}

impl Default for ColorValue {
    fn default() -> Self {
        Self::String(String::new())
    }
}

impl Serialize for ColorValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ColorValue::String(s) => serializer.serialize_str(s),
            ColorValue::Mapping(map) => {
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    m.serialize_entry(k, v)?;
                }
                m.end()
            },
            ColorValue::Sequence(seq) => {
                let mut s = serializer.serialize_seq(Some(seq.len()))?;
                for item in seq {
                    s.serialize_element(item)?;
                }
                s.end()
            },
        }
    }
}

impl<'de> Deserialize<'de> for ColorValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ColorsVisitor;

        impl<'de> Visitor<'de> for ColorsVisitor {
            type Value = ColorValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string, map, or sequence")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<ColorValue, E> {
                Ok(ColorValue::String(v.to_owned()))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<ColorValue, A::Error> {
                let mut result = Vec::new();
                while let Some(v) = seq.next_element::<String>()? {
                    result.push(v);
                }
                Ok(ColorValue::Sequence(result))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<ColorValue, A::Error> {
                let mut result = BTreeMap::new();
                while let Some((k, v)) = map.next_entry::<String, ColorValue>()? {
                    result.insert(k, v);
                }
                Ok(ColorValue::Mapping(result))
            }
        }

        deserializer.deserialize_any(ColorsVisitor)
    }
}

fn update_colors(colors: &mut BTreeMap<String, ColorValue>, palette: &BTreeMap<String, String>) {
    fn resolve(color: &str, palette: &BTreeMap<String, String>) -> String {
        color
            .split(':')
            .map(|c| palette.get(c).map_or(c, |s| s.as_str()))
            .collect::<Vec<&str>>()
            .join(":")
    }

    for color in colors.values_mut() {
        match color {
            ColorValue::String(s) => *s = resolve(s, palette),
            ColorValue::Mapping(map) => update_colors(map, palette),
            ColorValue::Sequence(seq) => {
                for v in seq {
                    *v = resolve(v, palette);
                }
            },
        }
    }
}
