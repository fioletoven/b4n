use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::define_symbols_set;

define_symbols_set! {
    left_sep:  '', '', '▒';
    right_sep: '', '', '▒';
    left_end: '', '', '░';
    right_end: '', '', '░';
    filtered: '', '▽', '▪';
    pinned: '󰐃', '▼', '▾';
    pod: '', '▤';
    container: '', '▭';
}

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

#[derive(Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Symbols {
    #[default]
    NerdFont,
    Powerline,
    Plain,
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
