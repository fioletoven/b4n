use super::*;

#[test]
fn from_str_test() {
    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::Char('+')), "+".into());
    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::Char('+')), "++".into());
    assert_eq!(KeyCombination::new(KeyModifiers::CONTROL, KeyCode::Char('+')), "Ctrl+".into());
    assert_eq!(KeyCombination::new(KeyModifiers::ALT, KeyCode::Char('+')), "Alt++".into());
    assert_eq!(KeyCombination::new(KeyModifiers::ALT, KeyCode::Char('+')), "Alt++++".into());
    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::Char('+')), "unknown+".into());

    assert_eq!(KeyCombination::new(KeyModifiers::ALT, KeyCode::Char('d')), "Alt+D".into());
    assert_eq!(KeyCombination::new(KeyModifiers::SHIFT, KeyCode::Char('e')), "SHIFT+e".into());
    assert_eq!(KeyCombination::new(KeyModifiers::SHIFT, KeyCode::Char('?')), "shift+?".into());

    assert_eq!(
        KeyCombination::new(KeyModifiers::ALT | KeyModifiers::SHIFT, KeyCode::Char('w')),
        "shift+ALT+W".into()
    );

    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::Null), "".into());
    assert_eq!(KeyCombination::new(KeyModifiers::NONE, KeyCode::Null), "unknown+aa".into());
    assert_eq!(
        KeyCombination::new(KeyModifiers::NONE, KeyCode::Char('z')),
        "unknown+z".into()
    );

    assert_eq!(KeyCombination::new(KeyModifiers::ALT, KeyCode::Home), "alt+home".into());
    assert_eq!(KeyCombination::new(KeyModifiers::CONTROL, KeyCode::Up), "control+up".into());
    assert_eq!(
        KeyCombination::new(KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Left),
        "alt+Shift+control+LEFT".into()
    );
}
