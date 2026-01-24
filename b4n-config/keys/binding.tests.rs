use std::iter::once;

use super::*;

#[test]
fn serialize_bindings_test() {
    let bindings = KeyBindings::default();
    let serialized = serde_yaml::to_string(&bindings).unwrap();
    let deserialized: KeyBindings = serde_yaml::from_str(&serialized).unwrap();

    assert_eq!(bindings, deserialized);
}

#[test]
fn merge_bindings_test() {
    let bindings = KeyBindings::default();
    assert_eq!(
        bindings.bindings[&"Ctrl+C".into()],
        once(KeyCommand::ApplicationExit).collect()
    );

    let other = KeyBindings::empty()
        .with("Ctrl+C", KeyCommand::FilterOpen)
        .with("Alt+A", KeyCommand::NavigateComplete);
    let bindings = KeyBindings::default_with(Some(other));

    assert!(bindings.bindings.contains_key(&"Ctrl+C".into()));
    assert_eq!(
        bindings.bindings[&"Ctrl+C".into()],
        [KeyCommand::FilterOpen, KeyCommand::ApplicationExit].into_iter().collect()
    );

    assert!(bindings.bindings.contains_key(&"Alt+A".into()));
    assert_eq!(
        bindings.bindings[&"Alt+A".into()],
        once(KeyCommand::NavigateComplete).collect()
    );
}

#[test]
fn has_binding_test() {
    let bindings = KeyBindings::default();
    assert!(bindings.has_binding(&"Ctrl+C".into(), KeyCommand::ApplicationExit));
    assert!(!bindings.has_binding(&"Ctrl+C".into(), KeyCommand::SearchOpen));
    assert!(!bindings.has_binding(&"Ctrl+D".into(), KeyCommand::ApplicationExit));

    let other = KeyBindings::empty().with("Ctrl+A", KeyCommand::NavigateDelete);
    assert!(other.has_binding(&"Ctrl+A".into(), KeyCommand::NavigateDelete));
    assert!(!other.has_binding(&"Ctrl+A".into(), KeyCommand::FilterReset));
    assert!(!other.has_binding(&"Ctrl+B".into(), KeyCommand::NavigateDelete));
}

#[test]
fn command_from_str_test() {
    assert!(KeyCommand::from_str("").is_err());
    assert!(KeyCommand::from_str("unknown").is_err());

    assert_eq!(KeyCommand::ApplicationExit, KeyCommand::from_str("app.exit").unwrap());
    assert_eq!(KeyCommand::FilterOpen, KeyCommand::from_str("filter.open").unwrap());
}

#[test]
fn serialize_command_test() {
    let key = serde_yaml::to_string(&KeyCommand::ApplicationExit).unwrap();
    assert_eq!("app.exit", key.trim());
}

#[test]
fn deserialize_command_test() {
    assert_eq!(
        KeyCommand::CommandPaletteOpen,
        serde_yaml::from_str("command-palette.open").unwrap()
    );
}
