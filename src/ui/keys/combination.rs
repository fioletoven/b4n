use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display, Write};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

#[cfg(test)]
#[path = "./combination.tests.rs"]
mod combination_tests;

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

/// Represents a specific key combination (key code + modifiers) used in a UI key binding.
#[derive(Debug, Clone, Copy)]
pub struct KeyCombination {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl PartialEq for KeyCombination {
    fn eq(&self, other: &Self) -> bool {
        if self.modifiers != other.modifiers {
            return false;
        }

        if let KeyCode::Char(sch) = self.code
            && let KeyCode::Char(och) = other.code
        {
            KeyCode::Char(sch.to_ascii_uppercase()) == KeyCode::Char(och.to_ascii_uppercase())
        } else {
            self.code == other.code
        }
    }
}

impl Eq for KeyCombination {}

impl Hash for KeyCombination {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let KeyCode::Char(ch) = self.code {
            KeyCode::Char(ch.to_ascii_uppercase()).hash(state);
            self.modifiers.hash(state);
        } else {
            self.code.hash(state);
            self.modifiers.hash(state);
        }
    }
}

impl Display for KeyCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: can be changed to KeyModifiers.to_string() when upgrades to crossterm 0.29
        if !self.modifiers.is_empty() {
            let mut first = true;

            for modifier in self.modifiers.iter() {
                if !first {
                    f.write_char('+')?;
                }

                first = false;
                match modifier {
                    KeyModifiers::SHIFT => f.write_str("Shift")?,
                    KeyModifiers::ALT => f.write_str("Alt")?,
                    KeyModifiers::CONTROL => f.write_str("Ctrl")?,
                    _ => (),
                };
            }

            f.write_char('+')?
        }

        let code = if let KeyCode::Char(c) = self.code {
            KeyCode::Char(c.to_ascii_uppercase())
        } else {
            self.code
        };
        write!(f, "{}", code)
    }
}

impl From<KeyCombination> for KeyEvent {
    fn from(value: KeyCombination) -> Self {
        KeyEvent::new(value.code, value.modifiers)
    }
}

impl From<KeyEvent> for KeyCombination {
    fn from(value: KeyEvent) -> Self {
        KeyCombination::new(value.code, value.modifiers)
    }
}

impl From<KeyCode> for KeyCombination {
    fn from(value: KeyCode) -> Self {
        KeyCombination::new(value, KeyModifiers::NONE)
    }
}

impl From<char> for KeyCombination {
    fn from(value: char) -> Self {
        KeyCombination::new(KeyCode::Char(value), KeyModifiers::NONE)
    }
}

impl From<&str> for KeyCombination {
    fn from(value: &str) -> Self {
        KeyCombination::from_str(value).unwrap_or_else(|_| KeyCombination::new(KeyCode::Null, KeyModifiers::NONE))
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
                KeyCode::Char(value.chars().next().unwrap().to_ascii_uppercase()),
                KeyModifiers::NONE,
            ));
        } else if value.contains("+++") {
            return Err(KeyCombinationError::UnknownCode);
        }

        let plus = value.ends_with("++");
        let elements = value.split('+').filter(|s| !s.is_empty()).collect::<Vec<_>>();

        if elements.is_empty() {
            Err(KeyCombinationError::UnknownModifier)
        } else if elements.len() == 1 && plus {
            Ok(KeyCombination::try_from(&elements, "+")?)
        } else if elements.len() == 1 {
            Ok(KeyCombination::try_from(&[], elements[0])?)
        } else {
            let len = elements.len().saturating_sub(1);
            Ok(KeyCombination::try_from(&elements[..len], elements[len])?)
        }
    }
}

impl KeyCombination {
    /// Creates new [`KeyCombination`] instance.
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Tries to create new [`KeyCombination`] instance from `modifiers` and `code` strings.
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
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeyCombination {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
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

        deserializer.deserialize_str(KeyCombinationVisitor)
    }
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
