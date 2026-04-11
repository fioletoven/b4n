use b4n_tui::MouseEventKind;
use crossterm::event::{KeyCode, KeyModifiers};

/// Converts a key event to terminal byte sequence.
pub fn encode_key(code: KeyCode, modifiers: KeyModifiers, app_mode: bool) -> Option<Vec<u8>> {
    match code {
        KeyCode::Char(input) => Some(get_char_bytes(input, modifiers)),

        KeyCode::Esc => Some(vec![27]),
        KeyCode::Backspace => Some(vec![127]),
        KeyCode::Enter => Some(vec![13]),
        KeyCode::PageUp => Some(vec![27, 91, 53, 126]),
        KeyCode::PageDown => Some(vec![27, 91, 54, 126]),
        KeyCode::Tab => Some(vec![9]),
        KeyCode::BackTab => Some(vec![27, 91, 90]),
        KeyCode::Delete => Some(vec![27, 91, 51, 126]),
        KeyCode::Insert => Some(vec![27, 91, 50, 126]),

        KeyCode::Home => Some(if app_mode { vec![27, 79, 72] } else { vec![27, 91, 72] }),
        KeyCode::End => Some(if app_mode { vec![27, 79, 70] } else { vec![27, 91, 70] }),

        KeyCode::Left => Some(if app_mode { vec![27, 79, 68] } else { vec![27, 91, 68] }),
        KeyCode::Right => Some(if app_mode { vec![27, 79, 67] } else { vec![27, 91, 67] }),
        KeyCode::Up => Some(if app_mode { vec![27, 79, 65] } else { vec![27, 91, 65] }),
        KeyCode::Down => Some(if app_mode { vec![27, 79, 66] } else { vec![27, 91, 66] }),

        KeyCode::F(n) => get_function_key_sequence(n, modifiers),

        _ => None,
    }
}

/// Encodes mouse event to SGR extended format.
pub fn encode_mouse(kind: MouseEventKind, column: u16, row: u16, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    let x = column + 1;
    let y = row + 1;

    let (button, is_release) = match kind {
        MouseEventKind::LeftClick => (0, false),
        MouseEventKind::LeftUp => (0, true),
        MouseEventKind::MiddleClick => (1, false),
        MouseEventKind::MiddleUp => (1, true),
        MouseEventKind::RightClick => (2, false),
        MouseEventKind::RightUp => (2, true),
        MouseEventKind::LeftDrag => (32, false),
        MouseEventKind::MiddleDrag => (33, false),
        MouseEventKind::RightDrag => (34, false),
        MouseEventKind::ScrollUp => (64, false),
        MouseEventKind::ScrollDown => (65, false),
        MouseEventKind::Moved => (35, false),
        _ => return None,
    };

    let mut button_code = button;
    if modifiers.contains(KeyModifiers::SHIFT) {
        button_code += 4;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        button_code += 8;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        button_code += 16;
    }

    let action = if is_release { 'm' } else { 'M' };

    Some(format!("\x1b[<{};{};{}{}", button_code, x, y, action).into_bytes())
}

/// Converts a character to bytes, handling CTRL modifier.
fn get_char_bytes(input: char, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers == KeyModifiers::CONTROL {
        let mut result = input.to_ascii_uppercase().to_string().into_bytes();
        result[0] = result[0].saturating_sub(64);
        result
    } else {
        input.to_string().into_bytes()
    }
}

/// Converts function key to terminal byte sequence.
fn get_function_key_sequence(n: u8, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    let modifier_code = get_modifier_code(modifiers);

    match n {
        1..=4 => {
            let key_char = b'P' + (n - 1);
            if modifier_code > 0 {
                Some(vec![27, 91, 49, 59, modifier_code, key_char])
            } else {
                Some(vec![27, 79, key_char])
            }
        },
        5..=12 => {
            let base = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => unreachable!(),
            };

            if modifier_code > 0 {
                Some(format!("\x1b[{};{}~", base, modifier_code).into_bytes())
            } else {
                Some(format!("\x1b[{}~", base).into_bytes())
            }
        },
        _ => None,
    }
}

/// Converts key modifiers to terminal modifier code.
fn get_modifier_code(modifiers: KeyModifiers) -> u8 {
    let mut code = 1;

    if modifiers.contains(KeyModifiers::SHIFT) {
        code += 1;
    }

    if modifiers.contains(KeyModifiers::ALT) {
        code += 2;
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        code += 4;
    }

    if code > 1 { code } else { 0 }
}
