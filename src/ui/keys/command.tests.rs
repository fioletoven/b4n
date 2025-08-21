use super::*;

#[test]
fn from_str_test() {
    assert!(KeyCommand::from_str("").is_err());
    assert!(KeyCommand::from_str("unknown").is_err());

    assert_eq!(KeyCommand::ApplicationExit, KeyCommand::from_str("app.exit").unwrap());
    assert_eq!(KeyCommand::FilterOpen, KeyCommand::from_str("filter.open").unwrap());
}

#[test]
fn serialize_test() {
    let key = serde_yaml::to_string(&KeyCommand::ApplicationExit).unwrap();
    assert_eq!("app.exit", key.trim());
}

#[test]
fn deserialize_test() {
    assert_eq!(
        KeyCommand::CommandPaletteOpen,
        serde_yaml::from_str("command-palette.open").unwrap()
    );
}
