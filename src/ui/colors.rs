use ratatui::style::Color;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self};
use std::str::FromStr;

/// Represents foreground and background colors for text.
#[derive(Default, Copy, Clone)]
pub struct TextColors {
    pub fg: Color,
    pub bg: Color,
}

impl TextColors {
    /// Returns new [`TextColors`] instance.
    pub fn new(fg: Color, bg: Color) -> Self {
        TextColors { fg, bg }
    }
}

impl Serialize for TextColors {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let text_colors = format!("{}:{}", self.fg, self.bg);
        serializer.serialize_str(&text_colors)
    }
}

impl<'de> Deserialize<'de> for TextColors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TextColorsVisitor)
    }
}

/// Internal [`Visitor`] for deserializing [`TextColors`].
struct TextColorsVisitor;

impl Visitor<'_> for TextColorsVisitor {
    type Value = TextColors;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing two colors separated by a comma")
    }

    fn visit_str<E>(self, value: &str) -> Result<TextColors, E>
    where
        E: de::Error,
    {
        let parts: Vec<&str> = value.split(':').collect();

        if parts.len() != 2 {
            return Err(de::Error::invalid_length(parts.len(), &self));
        }

        let Ok(fg) = Color::from_str(parts[0].trim()) else {
            return Err(de::Error::custom(format_args!("invalid color value: {}", parts[0])));
        };

        let Ok(bg) = Color::from_str(parts[1].trim()) else {
            return Err(de::Error::custom(format_args!("invalid color value: {}", parts[1])));
        };

        Ok(TextColors { fg, bg })
    }
}

/// Represents colors for text line.
#[derive(Default, Serialize, Deserialize, Copy, Clone)]
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
            if is_active {
                self.selected_hl
            } else {
                self.selected
            }
        } else if is_active {
            self.normal_hl
        } else {
            self.normal
        }
    }
}
