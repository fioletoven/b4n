use kube::api::TerminalSize;
use ratatui::layout::Rect;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use tui_term::vt100;

const ESC: u8 = 0x1B;

/// Detects terminal mode changes (application cursor keys, mouse) from raw PTY output.
pub fn detect_terminal_modes(data: &[u8]) -> (Option<bool>, Option<bool>) {
    const CSI_PRIVATE: &[u8] = &[ESC, b'[', b'?']; // ESC [ ?

    let mut application_mode = None;
    let mut mouse_mode = None;
    let mut i = 0;

    while let Some(slice) = data.get(i..) {
        let Some(offset) = slice.windows(CSI_PRIVATE.len()).position(|w| w == CSI_PRIVATE) else {
            break;
        };
        i += offset + CSI_PRIVATE.len();

        // collect digits and ';' until terminator 'h' or 'l'
        let params_start = i;
        let terminator = loop {
            match data.get(i) {
                Some(&b'h') => break b'h',
                Some(&b'l') => break b'l',
                Some(&b) if b.is_ascii_digit() || b == b';' => i += 1,
                _ => break 0,
            }
        };

        if terminator == 0 {
            break;
        }

        let enabled = terminator == b'h';
        for param in data[params_start..i].split(|&b| b == b';') {
            match param {
                b"1" => application_mode = Some(enabled),
                b"1000" | b"1002" | b"1003" | b"1006" => mouse_mode = Some(enabled),
                _ => {},
            }
        }

        i += 1;
    }

    (application_mode, mouse_mode)
}

/// Detects terminal mode changes and then updates it in [`TerminalState`].
pub fn update_terminal_state(data: &[u8], state: &mut TerminalState) {
    let (app_mode, mouse) = detect_terminal_modes(data);
    if let Some(enabled) = app_mode {
        state.set_cursor_key_mode(if enabled { 2 } else { 1 });
    }
    if let Some(enabled) = mouse {
        state.set_mouse_mode(if enabled { 2 } else { 1 });
    }
}

/// Responds to terminal queries embedded in raw process output.
pub fn handle_terminal_queries(data: &[u8], parser: &Arc<RwLock<vt100::Parser>>, size: &TerminalSize) -> Vec<u8> {
    let mut response = Vec::new();
    let mut i = 0;

    while i < data.len() {
        if data[i] != ESC {
            i += 1;
            continue;
        }

        let tail = &data[i..];

        // ESC Z case
        if tail.get(1) == Some(&b'Z') {
            response.extend_from_slice(b"\x1b[?6c");
            i += 2;
            continue;
        }

        // all ESC [ cases
        let Some(seq) = tail.get(2..) else { break };
        if tail.get(1) != Some(&b'[') {
            i += 1;
            continue;
        }

        if let Some(advance) = handle_csi(seq, &mut response, parser, size) {
            i += 2 + advance;
        } else {
            i += 1;
        }
    }

    response
}

/// Handles a CSI sequence (the part after `ESC [`).\
/// Returns how many bytes were consumed (including the final byte), or `None` to skip.
fn handle_csi(seq: &[u8], response: &mut Vec<u8>, parser: &Arc<RwLock<vt100::Parser>>, size: &TerminalSize) -> Option<usize> {
    Some(match seq {
        // ESC [ 6 n - cursor position query
        [b'6', b'n', ..] => {
            let (row, col) = cursor_position(parser);
            let _ = write!(response, "\x1b[{row};{col}R");
            2
        },

        // ESC [ 5 n - device status report, return "terminal OK"
        [b'5', b'n', ..] => {
            response.extend_from_slice(b"\x1b[0n");
            2
        },

        // ESC [ c  or  ESC [ 0 c - primary DA (DA1), identify as VT220
        [b'c', ..] => {
            response.extend_from_slice(b"\x1b[?62;22c");
            1
        },
        [b'0', b'c', ..] => {
            response.extend_from_slice(b"\x1b[?62;22c");
            2
        },

        // ESC [ > c  or  ESC [ > 0 c - secondary DA (DA2)
        [b'>', b'c', ..] => {
            response.extend_from_slice(b"\x1b[>1;10;0c");
            2
        },
        [b'>', b'0', b'c', ..] => {
            response.extend_from_slice(b"\x1b[>1;10;0c");
            3
        },

        // ESC [ 1 8 t - report terminal size
        [b'1', b'8', b't', ..] => {
            let _ = write!(response, "\x1b[8;{};{}t", size.height, size.width);
            3
        },

        _ => return None,
    })
}

fn cursor_position(parser: &Arc<RwLock<vt100::Parser>>) -> (u16, u16) {
    parser.read().map_or((1, 1), |p| {
        let (r, c) = p.screen().cursor_position();
        (r + 1, c + 1)
    })
}

/// Holds current terminal state.
#[derive(Clone)]
pub struct TerminalState {
    is_running: Arc<AtomicBool>,
    has_error: Arc<AtomicBool>,
    cursor_key_mode: Arc<AtomicU8>,
    mouse_mode: Arc<AtomicU8>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            has_error: Arc::new(AtomicBool::new(false)),
            cursor_key_mode: Arc::new(AtomicU8::new(0)),
            mouse_mode: Arc::new(AtomicU8::new(0)),
        }
    }
}

impl TerminalState {
    /// Returns `true` if terminal is running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Sets flag indicating if terminal is running.
    pub fn set_running(&mut self, is_running: bool) {
        self.is_running.store(is_running, Ordering::Relaxed);
    }

    /// Returns `true` if terminal has error.
    pub fn has_error(&self) -> bool {
        self.has_error.load(Ordering::Relaxed)
    }

    /// Sets flag indicating if terminal has error.
    pub fn set_error(&mut self, has_error: bool) {
        self.has_error.store(has_error, Ordering::Relaxed);
    }

    /// Gets the terminal cursor key mode.\
    /// 0 - unknown,\
    /// 1 - normal cursor key mode,\
    /// 2 - application cursor key mode.
    pub fn cursor_key_mode(&self) -> u8 {
        self.cursor_key_mode.load(Ordering::Relaxed)
    }

    /// Sets terminal cursor key mode.\
    /// 0 - unknown,\
    /// 1 - normal cursor key mode,\
    /// 2 - application cursor key mode.
    pub fn set_cursor_key_mode(&mut self, mode: u8) {
        self.cursor_key_mode.store(mode, Ordering::Relaxed);
    }

    /// Gets mouse mode.\
    /// 0 - unknown,\
    /// 1 - disabled,\
    /// 2 - enabled.
    pub fn mouse_mode(&self) -> u8 {
        self.mouse_mode.load(Ordering::Relaxed)
    }

    /// Sets mouse mode.\
    /// 0 - unknown,\
    /// 1 - disabled,\
    /// 2 - enabled.
    pub fn set_mouse_mode(&mut self, mode: u8) {
        self.mouse_mode.store(mode, Ordering::Relaxed);
    }
}

/// Extension methods for [`Rect`].
pub trait RectExt {
    /// Converts [`Rect`] to [`TerminalSize`].
    fn to_terminal_size(&self) -> TerminalSize;
}

impl RectExt for Rect {
    fn to_terminal_size(&self) -> TerminalSize {
        TerminalSize {
            width: self.width,
            height: self.height,
        }
    }
}
