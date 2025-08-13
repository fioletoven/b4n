use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(test)]
#[path = "./key_bindings.tests.rs"]
mod key_bindings_tests;

#[derive(Debug, PartialEq)]
pub struct KeyCombination {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl From<KeyEvent> for KeyCombination {
    fn from(value: KeyEvent) -> Self {
        KeyCombination::new(value.modifiers, value.code)
    }
}

impl From<&str> for KeyCombination {
    fn from(value: &str) -> Self {
        if value.chars().count() == 1 {
            return KeyCombination::new(
                KeyModifiers::NONE,
                KeyCode::Char(value.chars().next().unwrap().to_ascii_lowercase()),
            );
        }

        let plus = value.ends_with('+');
        let elements = value.split('+').filter(|s| !s.is_empty()).collect::<Vec<_>>();

        if elements.is_empty() && plus {
            KeyCombination::new(KeyModifiers::NONE, KeyCode::Char('+'))
        } else if elements.is_empty() {
            KeyCombination::new(KeyModifiers::NONE, KeyCode::Null)
        } else if elements.len() == 1 && plus {
            KeyCombination::from(&[elements[0]], "+")
        } else if elements.len() == 1 {
            KeyCombination::from(&[], elements[0])
        } else {
            let len = elements.len().saturating_sub(1);
            KeyCombination::from(&elements[..len], elements[len])
        }
    }
}

impl KeyCombination {
    pub fn new(modifiers: KeyModifiers, code: KeyCode) -> Self {
        let code = if let KeyCode::Char(c) = code {
            KeyCode::Char(c.to_ascii_lowercase())
        } else {
            code
        };

        Self { code, modifiers }
    }

    pub fn from(modifiers: &[&str], code: &str) -> Self {
        let code = match code.chars().count() {
            0 => KeyCode::Null,
            1 => KeyCode::Char(code.chars().next().unwrap().to_ascii_lowercase()),
            _ => get_code_from_name(code),
        };

        let mut all_modifiers = KeyModifiers::NONE;
        for modifier in modifiers {
            all_modifiers |= get_modifier_from_name(modifier);
        }

        Self {
            code,
            modifiers: all_modifiers,
        }
    }
}

fn get_modifier_from_name(modifier: &str) -> KeyModifiers {
    match modifier.to_ascii_lowercase().as_str() {
        "shift" => KeyModifiers::SHIFT,
        "alt" => KeyModifiers::ALT,
        "ctrl" => KeyModifiers::CONTROL,
        "control" => KeyModifiers::CONTROL,
        _ => KeyModifiers::NONE,
    }
}

fn get_code_from_name(code: &str) -> KeyCode {
    match code.to_ascii_lowercase().as_str() {
        "backspace" => KeyCode::Backspace,
        "enter" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "esc" => KeyCode::Esc,
        _ => KeyCode::Null,
    }
}
