use std::collections::HashMap;

use crate::ui::{KeyCombination, KeyCommand};

pub struct KeyBindings {
    bindings: HashMap<KeyCombination, KeyCommand>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut result = HashMap::new();

        result.insert("Ctrl+C".into(), "exit-app".into());
        result.insert("Shift+:".into(), "command-palette.open".into());
        result.insert("/".into(), "filter.open".into());

        Self { bindings: result }
    }
}
