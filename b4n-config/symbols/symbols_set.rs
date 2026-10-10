use serde::{Deserialize, Serialize};

use crate::define_symbols_set;

define_symbols_set! {
    left_sep:  "", "", "▒";
    right_sep: "", "", "▒";
    left_end: "", "", "░";
    right_end: "", "", "░";
    filtered: "", "▽", "▪";
    pinned: "󰐃", "▼", "▾";
    pod: "", "▤";
    container: "", "▭";
}

#[derive(Default, Serialize, Deserialize, Clone)]
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
