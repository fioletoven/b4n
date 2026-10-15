use super::*;

#[test]
fn update_symbols_test() {
    use b4n_config::symbols::Symbols;

    for symbols in Symbols::ALL {
        let mut action = ActionItem::new("action", ActionItemKind::Action);
        action.update_icon(&symbols);
        let expected = if symbols.action.raw.is_empty() {
            None
        } else {
            Some(symbols.action.ch)
        };
        assert_eq!(action.icon, expected);

        let mut plugin = ActionItem::new("plugin", ActionItemKind::Plugin);
        plugin.update_icon(&symbols);
        let expected = if symbols.plugin.raw.is_empty() {
            None
        } else {
            Some(symbols.plugin.ch)
        };
        assert_eq!(plugin.icon, expected);

        for kind in [ActionItemKind::Default, ActionItemKind::Menu] {
            let mut item = ActionItem::new("item", kind);
            item.update_icon(&symbols);
            assert_eq!(item.icon, None);
        }

        for kind in [ActionItemKind::Action, ActionItemKind::Plugin] {
            let mut item = ActionItem::new("custom", kind).with_icon(Some('*'));
            item.update_icon(&symbols);
            assert_eq!(item.icon, Some('*'));
        }
    }
}

#[test]
fn get_name_test() {
    let action = ActionItem::new("some long text that should be truncated", ActionItemKind::Action);
    assert_eq!("some long text t".to_owned(), action.get_name(16));

    let action = action.with_icon(Some(''));
    assert_eq!("some long text ".to_owned(), action.get_name(16));

    let action = ActionItem::new("some long text that should be truncated", ActionItemKind::Action)
        .with_icon(Some(''))
        .with_description("descr");
    assert_eq!("some long text ".to_owned(), action.get_name(16));

    let action = ActionItem::new("some text", ActionItemKind::Action)
        .with_icon(Some(''))
        .with_description("descr");
    assert_eq!("some text ␝desc␝ ".to_owned(), action.get_name(16));

    let action = ActionItem::new("text", ActionItemKind::Action)
        .with_icon(Some(''))
        .with_description("descr");
    assert_eq!("text     ␝descr␝ ".to_owned(), action.get_name(16));

    let mut action = ActionItem::new("text", ActionItemKind::Action)
        .with_icon(Some(''))
        .with_description("descr");
    action.set_key(Some("Ctrl+C".to_owned()));
    assert_eq!("text ␝descr␝  ␝❬␝Ctrl+C␝❭␝ ".to_owned(), action.get_name(22));

    let action = ActionItem::menu(9, '', "delete ␝selected␝", "");
    assert_eq!(" delete ␝selected␝     ".to_owned(), action.get_name(22));

    let action = ActionItem::menu(9, '', "logs ␝previous␝", "");
    assert_eq!(" logs ␝previous␝".to_owned(), action.get_name(15));

    let mut action = ActionItem::menu(9, '', "close", "close");
    action.show_menu_icon(false);
    assert_eq!("close".to_owned(), action.get_name(5));
}
