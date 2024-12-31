use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use super::colors::{LineColors, TextColors};

/// Represents kubernetes resource colors
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct ResourceColors {
    pub ready: LineColors,
    pub in_progress: LineColors,
    pub terminating: LineColors,
    pub completed: LineColors,
}

/// Represents colors for button
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct ButtonColors {
    pub normal: TextColors,
    pub focused: TextColors,
}

/// Represents colors for modal dialogs
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct ModalColors {
    pub colors: TextColors,
    pub btn_delete: ButtonColors,
    pub btn_cancel: ButtonColors,
}

/// Represents colors for selector widget
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct SelectorColors {
    pub normal: TextColors,
    pub normal_hl: TextColors,
    pub input: TextColors,
}

/// All colors in theme
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct ThemeColors {
    pub context: TextColors,
    pub namespace: TextColors,
    pub resource: TextColors,
    pub count: TextColors,
    pub info: TextColors,
    pub disconnected: TextColors,
    pub header: TextColors,
    pub modal: ModalColors,
    pub selector: SelectorColors,
    pub line: ResourceColors,
}

/// Theme used in the application
#[derive(Serialize, Deserialize)]
pub struct Theme {
    pub colors: ThemeColors,
}

impl Default for Theme {
    /// Returns TUI default theme for the application
    fn default() -> Self {
        Theme {
            colors: ThemeColors {
                context: TextColors::new(Color::White, Color::Rgb(216, 0, 96)),
                namespace: TextColors::new(Color::DarkGray, Color::Rgb(253, 202, 79)),
                resource: TextColors::new(Color::DarkGray, Color::Rgb(92, 166, 227)),
                count: TextColors::new(Color::DarkGray, Color::Rgb(170, 217, 46)),
                info: TextColors::new(Color::White, Color::Rgb(153, 113, 195)),
                disconnected: TextColors::new(Color::White, Color::LightRed),
                header: TextColors::new(Color::Gray, Color::DarkGray),
                modal: ModalColors {
                    colors: TextColors::new(Color::Gray, Color::DarkGray),
                    btn_delete: ButtonColors {
                        normal: TextColors::new(Color::White, Color::DarkGray),
                        focused: TextColors::new(Color::White, Color::LightRed),
                    },
                    btn_cancel: ButtonColors {
                        normal: TextColors::new(Color::White, Color::DarkGray),
                        focused: TextColors::new(Color::White, Color::LightGreen),
                    },
                },
                selector: SelectorColors {
                    normal: TextColors::new(Color::Gray, Color::DarkGray),
                    normal_hl: TextColors::new(Color::DarkGray, Color::Gray),
                    input: TextColors::new(Color::Blue, Color::DarkGray),
                },
                line: ResourceColors {
                    ready: LineColors {
                        normal: TextColors::new(Color::LightBlue, Color::Reset),
                        normal_hl: TextColors::new(Color::DarkGray, Color::LightBlue),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                    in_progress: LineColors {
                        normal: TextColors::new(Color::Red, Color::Reset),
                        normal_hl: TextColors::new(Color::DarkGray, Color::LightRed),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                    terminating: LineColors {
                        normal: TextColors::new(Color::Magenta, Color::Reset),
                        normal_hl: TextColors::new(Color::DarkGray, Color::LightMagenta),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                    completed: LineColors {
                        normal: TextColors::new(Color::Gray, Color::Reset),
                        normal_hl: TextColors::new(Color::Gray, Color::DarkGray),
                        selected: TextColors::new(Color::LightGreen, Color::Reset),
                        selected_hl: TextColors::new(Color::DarkGray, Color::LightGreen),
                    },
                },
            },
        }
    }
}
