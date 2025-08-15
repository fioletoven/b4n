use super::*;

#[test]
fn from_str_test() {
    assert!(KeyCombination::from_str("").is_err());
    assert!(KeyCombination::from_str("++").is_err());
    assert!(KeyCombination::from_str("++++").is_err());
    assert!(KeyCombination::from_str("Ctrl").is_err());
    assert!(KeyCombination::from_str("Ctrl+").is_err());
    assert!(KeyCombination::from_str("Alt++").is_err());
    assert!(KeyCombination::from_str("Alt+++++").is_err());
    assert!(KeyCombination::from_str("unknown+").is_err());
    assert!(KeyCombination::from_str("unknown+aa").is_err());
    assert!(KeyCombination::from_str("unknown+z").is_err());

    assert_eq!(
        KeyCombination::new(KeyModifiers::NONE, KeyCode::Char('+')),
        KeyCombination::from_str("+").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyModifiers::ALT, KeyCode::Char('D')),
        KeyCombination::from_str("Alt+D").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyModifiers::SHIFT, KeyCode::Char('E')),
        KeyCombination::from_str("SHIFT+e").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyModifiers::SHIFT, KeyCode::Char('?')),
        KeyCombination::from_str("shift+?").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyModifiers::ALT | KeyModifiers::SHIFT, KeyCode::Char('W')),
        KeyCombination::from_str("shift+ALT+W").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyModifiers::ALT, KeyCode::Home),
        KeyCombination::from_str("alt+home").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyModifiers::CONTROL, KeyCode::Up),
        KeyCombination::from_str("control+up").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Left),
        KeyCombination::from_str("option+Shift+control+LEFT").unwrap()
    );
}

#[test]
fn serialize_test() {
    let key = serde_yaml::to_string(&KeyCombination::new(KeyModifiers::NONE, KeyCode::Null)).unwrap();
    assert_eq!("'Null'", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyModifiers::NONE, KeyCode::Char('a'))).unwrap();
    assert_eq!("A", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyModifiers::NONE, KeyCode::F(5))).unwrap();
    assert_eq!("F5", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyModifiers::SHIFT, KeyCode::Char('A'))).unwrap();
    assert_eq!("Shift+A", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(
        KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT,
        KeyCode::Char('z'),
    ))
    .unwrap();
    assert_eq!("Shift+Ctrl+Alt+Z", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyModifiers::SHIFT, KeyCode::Backspace)).unwrap();
    assert_eq!("Shift+Backspace", key.trim());
}

#[test]
fn deserialize_test() {
    let key = serde_yaml::from_str("'Null'").unwrap();
    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::Null), key);

    let key = serde_yaml::from_str("Ctrl+A").unwrap();
    assert_eq!(KeyCombination::new(KeyModifiers::CONTROL, KeyCode::Char('A')), key);

    let key = serde_yaml::from_str("shift+Ctrl+x").unwrap();
    assert_eq!(
        KeyCombination::new(KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Char('X')),
        key
    );

    let key = serde_yaml::from_str("LEFT").unwrap();
    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::Left), key);

    let key = serde_yaml::from_str("F7").unwrap();
    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::F(7)), key);

    let key = serde_yaml::from_str("Shift+F12").unwrap();
    assert_eq!(KeyCombination::new(KeyModifiers::SHIFT, KeyCode::F(12)), key);
}
