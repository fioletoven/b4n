use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::define_symbols_set;

define_symbols_set! {
    left_sep:  '', '', '▒';
    right_sep: '', '', '▒';
    left_end: '', '', '░';
    right_end: '', '', '░';
    error: '', '⨯', 'x';
    back: '󰕍', '←', '←';
    enter: '', '⏎', '→';
    hint: '', '☼', '☼';
    prompt: '', '❭', '❯';
    filter: '', '▼', '▼';
    pin: '󰐃', '•', '•';
    pod: '', '▤';
    container: '', '▢';
}

/// A single symbol representation.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub ch: char,
    pub raw: &'static str,
    pub right: &'static str,
}

impl std::ops::Deref for Symbol {
    type Target = str;
    fn deref(&self) -> &str {
        self.raw
    }
}

impl From<&Symbol> for Cow<'_, str> {
    fn from(value: &Symbol) -> Self {
        value.raw.into()
    }
}

/// Available symbol sets.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Symbols {
    #[default]
    NerdFont,
    Powerline,
    Plain,
}

impl Symbols {
    /// Returns all available symbol sets.
    pub const ALL: [Self; 3] = [Self::NerdFont, Self::Powerline, Self::Plain];

    /// Returns the string representation of the symbol set.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NerdFont => "nerdfont",
            Self::Powerline => "powerline",
            Self::Plain => "plain",
        }
    }
}

impl std::ops::Deref for Symbols {
    type Target = SymbolsSet;

    fn deref(&self) -> &'static SymbolsSet {
        match self {
            Symbols::NerdFont => &Symbols::NF,
            Symbols::Powerline => &Symbols::PL,
            Symbols::Plain => &Symbols::PLAIN,
        }
    }
}
