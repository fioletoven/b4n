use ratatui_core::style::{Color, Style};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self};
use std::str::FromStr;

/// Represents foreground, dim foreground and background colors for UI text.
#[derive(Default, Copy, Clone)]
pub struct TextColors {
    pub fg: Color,
    pub dim: Color,
    pub bg: Color,
}

impl TextColors {
    /// Returns new [`TextColors`] instance.
    pub fn new(fg: Color) -> Self {
        TextColors::dim(fg, Color::Reset, Color::Reset)
    }

    /// Returns new [`TextColors`] instance with `bg` color set.
    pub fn bg(fg: Color, bg: Color) -> Self {
        TextColors::dim(fg, Color::Reset, bg)
    }

    /// Returns new [`TextColors`] instance with `bg` and `dim` colors set.
    pub fn dim(fg: Color, dim: Color, bg: Color) -> Self {
        Self { fg, dim, bg }
    }

    /// Returns new [`TextColors`] instance reverting `fg` with `bg` from the current one.
    pub fn to_reverted(self) -> Self {
        Self {
            fg: self.bg,
            dim: self.dim,
            bg: self.fg,
        }
    }
}

impl From<&TextColors> for Style {
    fn from(value: &TextColors) -> Self {
        Style::default().fg(value.fg).bg(value.bg)
    }
}

impl Serialize for TextColors {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let text_colors = if self.bg == Color::Reset && self.dim == Color::Reset {
            format!("{}", self.fg)
        } else if self.dim == Color::Reset {
            format!("{}:{}", self.fg, self.bg)
        } else {
            format!("{}:{}:{}", self.fg, self.dim, self.bg)
        };
        serializer.serialize_str(&text_colors)
    }
}

impl<'de> Deserialize<'de> for TextColors {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TextColorsVisitor;

        impl Visitor<'_> for TextColorsVisitor {
            type Value = TextColors;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string containing 1-3 colors each separated by a comma")
            }

            fn visit_str<E>(self, value: &str) -> Result<TextColors, E>
            where
                E: de::Error,
            {
                let parts: Vec<&str> = value.split(':').collect();

                if parts.is_empty() || parts.len() > 3 {
                    return Err(de::Error::invalid_length(parts.len(), &self));
                }

                let Ok(col1) = Color::from_str(parts[0].trim()) else {
                    return Err(de::Error::custom(format_args!("invalid color value on pos 1: {}", parts[0])));
                };

                if parts.len() == 1 {
                    return Ok(TextColors::new(col1));
                }

                let Ok(col2) = Color::from_str(parts[1].trim()) else {
                    return Err(de::Error::custom(format_args!("invalid color value on pos 2: {}", parts[1])));
                };

                if parts.len() == 2 {
                    return Ok(TextColors::bg(col1, col2));
                }

                let Ok(col3) = Color::from_str(parts[2].trim()) else {
                    return Err(de::Error::custom(format_args!("invalid color value on pos 3: {}", parts[2])));
                };

                Ok(TextColors::dim(col1, col2, col3))
            }
        }

        deserializer.deserialize_str(TextColorsVisitor)
    }
}

/// Represents colors for text line.
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct LineColors {
    pub normal: TextColors,
    pub normal_hl: TextColors,
    pub selected: TextColors,
    pub selected_hl: TextColors,
}

impl LineColors {
    /// Returns [`TextColors`] for text line that reflects its state (normal, highlighted or selected).
    pub fn get_specific(&self, is_active: bool, is_selected: bool) -> TextColors {
        if is_selected {
            if is_active { self.selected_hl } else { self.selected }
        } else if is_active {
            self.normal_hl
        } else {
            self.normal
        }
    }
}

/// Converts syntect color to ratatui color.
pub fn from_syntect_color(syntect_color: syntect::highlighting::Color) -> Color {
    match syntect_color {
        syntect::highlighting::Color { r, g, b, a } if a > 2 => Color::Rgb(r, g, b),
        syntect::highlighting::Color { r, g: _, b: _, a: 2 } => Color::Indexed(r),
        syntect::highlighting::Color { r, g: _, b: _, a: 1 } => from_int_color(r),
        _ => Color::Reset,
    }
}

/// Converts ratatui color to syntect color.
pub fn to_syntect_color(ratatui_color: Color) -> syntect::highlighting::Color {
    match ratatui_color {
        Color::Reset => syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 0 },
        Color::Black => syntect::highlighting::Color { r: 1, g: 0, b: 0, a: 1 },
        Color::Red => syntect::highlighting::Color { r: 2, g: 0, b: 0, a: 1 },
        Color::Green => syntect::highlighting::Color { r: 3, g: 0, b: 0, a: 1 },
        Color::Yellow => syntect::highlighting::Color { r: 4, g: 0, b: 0, a: 1 },
        Color::Blue => syntect::highlighting::Color { r: 5, g: 0, b: 0, a: 1 },
        Color::Magenta => syntect::highlighting::Color { r: 6, g: 0, b: 0, a: 1 },
        Color::Cyan => syntect::highlighting::Color { r: 7, g: 0, b: 0, a: 1 },
        Color::Gray => syntect::highlighting::Color { r: 8, g: 0, b: 0, a: 1 },
        Color::DarkGray => syntect::highlighting::Color { r: 9, g: 0, b: 0, a: 1 },
        Color::LightRed => syntect::highlighting::Color { r: 10, g: 0, b: 0, a: 1 },
        Color::LightGreen => syntect::highlighting::Color { r: 11, g: 0, b: 0, a: 1 },
        Color::LightYellow => syntect::highlighting::Color { r: 12, g: 0, b: 0, a: 1 },
        Color::LightBlue => syntect::highlighting::Color { r: 13, g: 0, b: 0, a: 1 },
        Color::LightMagenta => syntect::highlighting::Color { r: 14, g: 0, b: 0, a: 1 },
        Color::LightCyan => syntect::highlighting::Color { r: 15, g: 0, b: 0, a: 1 },
        Color::White => syntect::highlighting::Color { r: 16, g: 0, b: 0, a: 1 },
        Color::Rgb(r, g, b) => syntect::highlighting::Color { r, g, b, a: 255 },
        Color::Indexed(i) => syntect::highlighting::Color { r: i, g: 0, b: 0, a: 2 },
    }
}

fn from_int_color(color: u8) -> Color {
    match color {
        1 => Color::Black,
        2 => Color::Red,
        3 => Color::Green,
        4 => Color::Yellow,
        5 => Color::Blue,
        6 => Color::Magenta,
        7 => Color::Cyan,
        8 => Color::Gray,
        9 => Color::DarkGray,
        10 => Color::LightRed,
        11 => Color::LightGreen,
        12 => Color::LightYellow,
        13 => Color::LightBlue,
        14 => Color::LightMagenta,
        15 => Color::LightCyan,
        16 => Color::White,
        _ => Color::Reset,
    }
}
