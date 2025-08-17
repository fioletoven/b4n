use super::*;

#[test]
fn serialize_test() {
    let bindings = KeyBindings::default();
    let serialized = serde_yaml::to_string(&bindings).unwrap();
    let deserialized: KeyBindings = serde_yaml::from_str(&serialized).unwrap();

    assert_eq!(bindings, deserialized);
}

#[test]
fn merge_test() {
    let bindings = KeyBindings::default();
    assert_eq!(bindings.bindings[&"Ctrl+C".into()], "exit-app".into());

    let mut other = KeyBindings::empty();
    other.insert("Ctrl+C", "yaml.open");
    other.insert("Alt+A", "exit-app");

    let bindings = KeyBindings::default_with(Some(other));

    assert!(bindings.bindings.contains_key(&"Ctrl+C".into()));
    assert_eq!(bindings.bindings[&"Ctrl+C".into()], "yaml.open".into());

    assert!(bindings.bindings.contains_key(&"Alt+A".into()));
    assert_eq!(bindings.bindings[&"Alt+A".into()], "exit-app".into());
}

#[test]
fn has_binding_test() {
    let bindings = KeyBindings::default();
    assert!(bindings.has_binding(&"Ctrl+C".into(), CommandTarget::Application, CommandAction::Exit));
    assert!(!bindings.has_binding(&"Ctrl+C".into(), CommandTarget::Application, CommandAction::Open));
    assert!(!bindings.has_binding(&"Ctrl+D".into(), CommandTarget::Application, CommandAction::Exit));

    let mut other = KeyBindings::empty();
    other.insert("Ctrl+A", "yaml.describe");
    assert!(other.has_binding(
        &"Ctrl+A".into(),
        CommandTarget::view("yaml"),
        CommandAction::action("describe")
    ));
    assert!(!other.has_binding(
        &"Ctrl+A".into(),
        CommandTarget::view("yaml"),
        CommandAction::action("not-describe")
    ));
    assert!(!other.has_binding(
        &"Ctrl+B".into(),
        CommandTarget::view("yaml"),
        CommandAction::action("describe")
    ));
}
