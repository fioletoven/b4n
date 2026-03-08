use b4n_common::sanitize_and_split;
use tokio::sync::mpsc;

use super::*;

#[test]
fn insert_text_internal_test() {
    let (request_tx, _) = mpsc::unbounded_channel::<HighlightRequest>();
    let mut content = YamlContent::new(Vec::new(), Vec::new(), request_tx, false, StyleFallback::default());
    content.insert_text_internal(ContentPosition::new(0, 0), vec!["first".to_owned(), "second".to_owned()]);
    assert_eq!("first\nsecond", content.plain.join("\n"));

    content.insert_text_internal(ContentPosition::new(0, 1), sanitize_and_split("\n"));
    assert_eq!("first\n\nsecond", content.plain.join("\n"));
}

#[test]
fn to_plain_text_test() {
    let (request_tx, _) = mpsc::unbounded_channel::<HighlightRequest>();
    let mut content = YamlContent::new(Vec::new(), Vec::new(), request_tx, false, StyleFallback::default());
    content.insert_text_internal(ContentPosition::new(0, 0), vec!["first".to_owned(), "second".to_owned()]);

    let result = content.to_plain_text(Some(Selection {
        start: ContentPosition::new(5, 0),
        end: ContentPosition::new(5, 0),
    }));

    assert_eq!("\n", result);
}
