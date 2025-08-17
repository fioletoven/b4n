use super::*;

#[test]
fn from_str_test() {
    assert!(KeyCommand::from_str("").is_err());

    assert_eq!(
        KeyCommand::new(CommandTarget::Application, CommandAction::Search),
        KeyCommand::from_str("search").unwrap()
    );

    assert_eq!(
        KeyCommand::new(CommandTarget::Application, CommandAction::Exit),
        KeyCommand::from_str("app.exit").unwrap()
    );

    assert_eq!(
        KeyCommand::new(CommandTarget::CommandPalette, CommandAction::Open),
        KeyCommand::from_str("command-palette.open").unwrap()
    );

    assert_eq!(
        KeyCommand::new(CommandTarget::View("logs".into()), CommandAction::Search),
        KeyCommand::from_str("logs.search").unwrap()
    );
}

#[test]
fn serialize_test() {
    let key = serde_yaml::to_string(&KeyCommand::new(CommandTarget::Application, CommandAction::Close)).unwrap();
    assert_eq!("app.close", key.trim());

    let key = serde_yaml::to_string(&KeyCommand::new(CommandTarget::Filter, CommandAction::Open)).unwrap();
    assert_eq!("filter.open", key.trim());

    let key = serde_yaml::to_string(&KeyCommand::new(CommandTarget::View("yaml".into()), CommandAction::Search)).unwrap();
    assert_eq!("yaml.search", key.trim());
}

#[test]
fn deserialize_test() {
    assert_eq!(
        KeyCommand::new(CommandTarget::CommandPalette, CommandAction::Open),
        serde_yaml::from_str("command-palette.open").unwrap()
    );

    assert_eq!(
        KeyCommand::new(CommandTarget::View("yaml".into()), CommandAction::Search),
        serde_yaml::from_str("yaml.search").unwrap()
    );
}
