use super::*;

#[test]
fn get_name_test() {
    let action = ActionItem::new("some long text that should be truncated");
    assert_eq!("some long text ".to_owned(), action.get_name(16));

    let action = ActionItem::new("some long text that should be truncated").with_description("descr");
    assert_eq!("some long text ".to_owned(), action.get_name(16));

    let action = ActionItem::new("some text").with_description("descr");
    assert_eq!("some text [desc] ".to_owned(), action.get_name(16));

    let action = ActionItem::new("text").with_description("descr");
    assert_eq!("text     [descr] ".to_owned(), action.get_name(16));
}
