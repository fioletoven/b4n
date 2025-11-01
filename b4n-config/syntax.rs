use syntect::{dumps::from_uncompressed_data, parsing::SyntaxSet};

use crate::themes::Theme;

pub const SYNTAX_SET_DATA: &[u8] = include_bytes!("../assets/syntaxes/syntaxes.packdump");

/// Keeps data required for syntax highlighting.
pub struct SyntaxData {
    pub syntax_set: SyntaxSet,
    pub yaml_theme: syntect::highlighting::Theme,
}

impl SyntaxData {
    /// Creates new [`SyntaxData`] instance.
    pub fn new(theme: &Theme) -> SyntaxData {
        SyntaxData {
            syntax_set: from_uncompressed_data::<SyntaxSet>(SYNTAX_SET_DATA).expect("cannot load SyntaxSet"),
            yaml_theme: theme.build_syntect_yaml_theme(),
        }
    }
}
