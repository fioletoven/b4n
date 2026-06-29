use b4n_tui::MouseEventKind;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;

/// Converts a key event to terminal byte sequence.
pub fn encode_key(code: KeyCode, modifiers: KeyModifiers, app_mode: bool) -> Option<Vec<u8>> {
    let mut bytes = match code {
        KeyCode::Char(input) => Some(get_char_bytes(input, modifiers)),

        KeyCode::Esc => Some(vec![27]),
        KeyCode::Backspace => Some(vec![127]),
        KeyCode::Enter => Some(vec![13]),
        KeyCode::PageUp => Some(get_tilde_key_sequence(5, modifiers)),
        KeyCode::PageDown => Some(get_tilde_key_sequence(6, modifiers)),
        KeyCode::Tab => Some(vec![9]),
        KeyCode::BackTab => Some(vec![27, 91, 90]),
        KeyCode::Delete => Some(vec![27, 91, 51, 126]),
        KeyCode::Insert => Some(vec![27, 91, 50, 126]),

        KeyCode::Home => Some(get_navigation_key_sequence(b'H', modifiers, app_mode)),
        KeyCode::End => Some(get_navigation_key_sequence(b'F', modifiers, app_mode)),

        KeyCode::Left => Some(get_navigation_key_sequence(b'D', modifiers, app_mode)),
        KeyCode::Right => Some(get_navigation_key_sequence(b'C', modifiers, app_mode)),
        KeyCode::Up => Some(get_navigation_key_sequence(b'A', modifiers, app_mode)),
        KeyCode::Down => Some(get_navigation_key_sequence(b'B', modifiers, app_mode)),

        KeyCode::F(n) => get_function_key_sequence(n, modifiers),

        _ => None,
    }?;

    if modifiers.contains(KeyModifiers::ALT) && !uses_csi_modifier_params(code, modifiers) {
        bytes.insert(0, 27);
    }

    Some(bytes)
}

/// Encodes mouse event to SGR extended format.
pub fn encode_mouse(kind: MouseEventKind, column: u16, row: u16, area: Rect, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    let (button, is_release) = match kind {
        MouseEventKind::LeftClick | MouseEventKind::LeftDoubleClick | MouseEventKind::LeftTripleClick => (0, false),
        MouseEventKind::LeftUp => (0, true),
        MouseEventKind::MiddleClick => (1, false),
        MouseEventKind::MiddleUp => (1, true),
        MouseEventKind::RightClick => (2, false),
        MouseEventKind::RightUp => (2, true),

        MouseEventKind::LeftDrag => (32, false),
        MouseEventKind::MiddleDrag => (33, false),
        MouseEventKind::RightDrag => (34, false),
        MouseEventKind::Moved => (35, false),

        MouseEventKind::ScrollUp => (64, false),
        MouseEventKind::ScrollDown => (65, false),
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

    let x = column.saturating_sub(area.x) + 1;
    let y = row.saturating_sub(area.y) + 1;
    let action = if is_release { 'm' } else { 'M' };

    Some(format!("\x1b[<{button_code};{x};{y}{action}").into_bytes())
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

/// Converts cursor, Home, and End keys to terminal byte sequences.
fn get_navigation_key_sequence(key_char: u8, modifiers: KeyModifiers, app_mode: bool) -> Vec<u8> {
    let modifier_code = get_modifier_code(modifiers);

    if modifier_code > 0 {
        format!("\x1b[1;{modifier_code}{}", char::from(key_char)).into_bytes()
    } else if app_mode {
        vec![27, 79, key_char]
    } else {
        vec![27, 91, key_char]
    }
}

/// Converts Fn, PageUp, and PageDown keys to terminal byte sequences.
fn get_tilde_key_sequence(base: u8, modifiers: KeyModifiers) -> Vec<u8> {
    let modifier_code = get_modifier_code(modifiers);

    if modifier_code > 0 {
        format!("\x1b[{base};{modifier_code}~").into_bytes()
    } else {
        format!("\x1b[{base}~").into_bytes()
    }
}

/// Converts function key to terminal byte sequence.
fn get_function_key_sequence(n: u8, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    match n {
        1..=4 => {
            let key_char = b'P' + (n - 1);
            let modifier_code = get_modifier_code(modifiers);

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

            Some(get_tilde_key_sequence(base, modifiers))
        },
        _ => None,
    }
}

fn uses_csi_modifier_params(code: KeyCode, modifiers: KeyModifiers) -> bool {
    if get_modifier_code(modifiers) == 0 {
        return false;
    }

    matches!(
        code,
        KeyCode::Home
            | KeyCode::End
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::F(_)
    )
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
