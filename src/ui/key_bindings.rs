use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self};
use std::str::FromStr;

#[cfg(test)]
#[path = "./key_bindings.tests.rs"]
mod key_bindings_tests;

/// Possible errors from [`KeyCombination`] parsing.
#[derive(thiserror::Error, Debug)]
pub enum KeyCombinationError {
    /// Unknown key modifier.
    #[error("unknown key modifier")]
    UnknownModifier,

    /// Unknown key code.
    #[error("unknown key code")]
    UnknownCode,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct KeyCombination {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl From<KeyEvent> for KeyCombination {
    fn from(value: KeyEvent) -> Self {
        KeyCombination::new(value.modifiers, value.code)
    }
}

impl FromStr for KeyCombination {
    type Err = KeyCombinationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let len = value.chars().count();
        if len == 0 {
            return Err(KeyCombinationError::UnknownCode);
        } else if len == 1 {
            return Ok(KeyCombination::new(
                KeyModifiers::NONE,
                KeyCode::Char(value.chars().next().unwrap().to_ascii_lowercase()),
            ));
        } else if value.contains("++") {
            return Err(KeyCombinationError::UnknownCode);
        }

        let plus = value.ends_with('+');
        let elements = value.split('+').filter(|s| !s.is_empty()).collect::<Vec<_>>();

        if elements.is_empty() {
            Err(KeyCombinationError::UnknownModifier)
        } else if elements.len() == 1 && plus {
            Err(KeyCombinationError::UnknownCode)
        } else if elements.len() == 1 {
            Ok(KeyCombination::try_from(&[], elements[0])?)
        } else {
            let len = elements.len().saturating_sub(1);
            Ok(KeyCombination::try_from(&elements[..len], elements[len])?)
        }
    }
}

impl KeyCombination {
    pub fn new(modifiers: KeyModifiers, code: KeyCode) -> Self {
        let code = if let KeyCode::Char(c) = code {
            KeyCode::Char(c.to_ascii_uppercase())
        } else {
            code
        };

        Self { code, modifiers }
    }

    pub fn try_from(modifiers: &[&str], code: &str) -> Result<Self, KeyCombinationError> {
        let code = match code.chars().count() {
            0 => KeyCode::Null,
            1 => KeyCode::Char(code.chars().next().unwrap().to_ascii_uppercase()),
            _ => get_code_from_name(code)?,
        };

        let mut all_modifiers = KeyModifiers::NONE;
        for modifier in modifiers {
            all_modifiers |= get_modifier_from_name(modifier)?;
        }

        Ok(Self {
            code,
            modifiers: all_modifiers,
        })
    }
}

impl Serialize for KeyCombination {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let modifiers = get_string_from_modifiers(&self.modifiers);
        if modifiers.is_empty() {
            serializer.serialize_str(&self.code.to_string())
        } else {
            serializer.serialize_str(&format!("{}+{}", modifiers, self.code))
        }
    }
}

impl<'de> Deserialize<'de> for KeyCombination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(KeyCombinationVisitor)
    }
}

/// Internal [`Visitor`] for deserializing [`KeyCombination`].
struct KeyCombinationVisitor;

impl Visitor<'_> for KeyCombinationVisitor {
    type Value = KeyCombination;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string containing key combination")
    }

    fn visit_str<E>(self, value: &str) -> Result<KeyCombination, E>
    where
        E: de::Error,
    {
        match KeyCombination::from_str(value) {
            Ok(key) => Ok(key),
            Err(_) => Err(de::Error::invalid_value(Unexpected::Str(value), &self)),
        }
    }
}

fn get_string_from_modifiers(modifiers: &KeyModifiers) -> String {
    let mut result = String::new();
    let mut first = true;

    for modifier in modifiers.iter() {
        if !first {
            result.push('+');
        }

        first = false;
        match modifier {
            KeyModifiers::SHIFT => result.push_str("Shift"),
            KeyModifiers::ALT => result.push_str("Alt"),
            KeyModifiers::CONTROL => result.push_str("Ctrl"),
            _ => (),
        };
    }

    result
}

fn get_modifier_from_name(modifier: &str) -> Result<KeyModifiers, KeyCombinationError> {
    match modifier.to_ascii_lowercase().as_str() {
        "shift" => Ok(KeyModifiers::SHIFT),
        "alt" => Ok(KeyModifiers::ALT),
        "option" => Ok(KeyModifiers::ALT),
        "ctrl" => Ok(KeyModifiers::CONTROL),
        "control" => Ok(KeyModifiers::CONTROL),
        _ => Err(KeyCombinationError::UnknownModifier),
    }
}

fn get_code_from_name(code: &str) -> Result<KeyCode, KeyCombinationError> {
    let code = code.to_ascii_lowercase();
    let code = code.as_str();
    if code.len() >= 2
        && code.len() <= 3
        && code.starts_with('f')
        && let Ok(num) = code[1..].parse()
    {
        if num > 0 && num <= 12 {
            return Ok(KeyCode::F(num));
        } else {
            return Err(KeyCombinationError::UnknownCode);
        }
    }

    match code {
        "backspace" => Ok(KeyCode::Backspace),
        "enter" => Ok(KeyCode::Enter),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" => Ok(KeyCode::PageUp),
        "pagedown" => Ok(KeyCode::PageDown),
        "tab" => Ok(KeyCode::Tab),
        "backtab" => Ok(KeyCode::BackTab),
        "delete" => Ok(KeyCode::Delete),
        "insert" => Ok(KeyCode::Insert),
        "esc" => Ok(KeyCode::Esc),
        "null" => Ok(KeyCode::Null),
        _ => Err(KeyCombinationError::UnknownCode),
    }
}
