pub use self::colors::{LineColors, TextColors, from_syntect_color, to_syntect_color};
pub use self::theme::{
    ControlColors, FilterColors, FooterColors, LogsSyntaxColors, ModalColors, ResourceColors, SelectColors, Theme, ThemeColors,
};

mod colors;
mod theme;
