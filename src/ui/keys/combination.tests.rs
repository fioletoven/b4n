use super::*;

#[test]
fn from_str_test() {
    assert!(KeyCombination::from_str("").is_err());
    assert!(KeyCombination::from_str("++").is_err());
    assert!(KeyCombination::from_str("++++").is_err());
    assert!(KeyCombination::from_str("Ctrl").is_err());
    assert!(KeyCombination::from_str("Alt+++").is_err());
    assert!(KeyCombination::from_str("Ctrl+").is_err());
    assert!(KeyCombination::from_str("Alt+++++").is_err());
    assert!(KeyCombination::from_str("unknown+").is_err());
    assert!(KeyCombination::from_str("unknown+aa").is_err());
    assert!(KeyCombination::from_str("unknown+z").is_err());

    assert_eq!(
        KeyCombination::new(KeyCode::Char('+'), KeyModifiers::NONE),
        KeyCombination::from_str("+").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Char('+'), KeyModifiers::ALT),
        KeyCombination::from_str("Alt++").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Char('D'), KeyModifiers::ALT),
        KeyCombination::from_str("Alt+D").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Char('E'), KeyModifiers::SHIFT),
        KeyCombination::from_str("SHIFT+e").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Char('?'), KeyModifiers::SHIFT),
        KeyCombination::from_str("shift+?").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Char('W'), KeyModifiers::ALT | KeyModifiers::SHIFT),
        KeyCombination::from_str("shift+ALT+W").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Home, KeyModifiers::ALT),
        KeyCombination::from_str("alt+home").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Up, KeyModifiers::CONTROL),
        KeyCombination::from_str("control+up").unwrap()
    );
    assert_eq!(
        KeyCombination::new(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SHIFT),
        KeyCombination::from_str("option+Shift+control+LEFT").unwrap()
    );
}

#[test]
fn serialize_test() {
    let key = serde_yaml::to_string(&KeyCombination::new(KeyCode::Null, KeyModifiers::NONE)).unwrap();
    assert_eq!("'Null'", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyCode::Char('a'), KeyModifiers::NONE)).unwrap();
    assert_eq!("A", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyCode::F(5), KeyModifiers::NONE)).unwrap();
    assert_eq!("F5", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyCode::Char('A'), KeyModifiers::SHIFT)).unwrap();
    assert_eq!("Shift+A", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT,
    ))
    .unwrap();
    assert_eq!("Shift+Ctrl+Alt+Z", key.trim());

    let key = serde_yaml::to_string(&KeyCombination::new(KeyCode::Backspace, KeyModifiers::SHIFT)).unwrap();
    assert_eq!("Shift+Backspace", key.trim());
}

#[test]
fn deserialize_test() {
    let key = serde_yaml::from_str("'Null'").unwrap();
    assert_eq!(KeyCombination::new(KeyCode::Null, KeyModifiers::NONE), key);

    let key = serde_yaml::from_str("Ctrl+A").unwrap();
    assert_eq!(KeyCombination::new(KeyCode::Char('A'), KeyModifiers::CONTROL), key);

    let key = serde_yaml::from_str("shift+Ctrl+x").unwrap();
    assert_eq!(
        KeyCombination::new(KeyCode::Char('X'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
        key
    );

    let key = serde_yaml::from_str("LEFT").unwrap();
    assert_eq!(KeyCombination::new(KeyCode::Left, KeyModifiers::NONE), key);

    let key = serde_yaml::from_str("F7").unwrap();
    assert_eq!(KeyCombination::new(KeyCode::F(7), KeyModifiers::NONE), key);

    let key = serde_yaml::from_str("Shift+F12").unwrap();
    assert_eq!(KeyCombination::new(KeyCode::F(12), KeyModifiers::SHIFT), key);
}
