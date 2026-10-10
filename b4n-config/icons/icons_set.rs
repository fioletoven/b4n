use serde::{Deserialize, Serialize};

use crate::define_icons_set;

define_icons_set! {
    left_sep:  "", "", "▒";
    right_sep: "", "", "▒";
    left_end: "", "", "░";
    right_end: "", "", "░";
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub enum Icons {
    NerdFont,
    Powerline,
    #[default]
    Plain,
}

impl std::ops::Deref for Icons {
    type Target = IconsSet;

    fn deref(&self) -> &'static IconsSet {
        match self {
            Icons::NerdFont => &Icons::NF,
            Icons::Powerline => &Icons::PL,
            Icons::Plain => &Icons::PLAIN,
        }
    }
}
