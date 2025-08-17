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
    let mut bindings = KeyBindings::default();

    let mut other = KeyBindings::empty();
    other.insert("Ctrl+C", "yaml.open");
    other.insert("Alt+A", "exit-app");

    assert_eq!(bindings.bindings[&"Ctrl+C".into()], "exit-app".into());

    bindings.merge(other);

    assert!(bindings.bindings.contains_key(&"Ctrl+C".into()));
    assert_eq!(bindings.bindings[&"Ctrl+C".into()], "yaml.open".into());

    assert!(bindings.bindings.contains_key(&"Alt+A".into()));
    assert_eq!(bindings.bindings[&"Alt+A".into()], "exit-app".into());
}
