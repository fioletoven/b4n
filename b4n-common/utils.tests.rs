use super::*;

#[test]
fn word_bounds_test() {
    let msg = "Hello";
    assert_eq!((0, 4), word_bounds(msg, 2).unwrap());

    let msg = "Hello world";
    assert_eq!((0, 4), word_bounds(msg, 2).unwrap());
    assert_eq!((6, 10), word_bounds(msg, 8).unwrap());

    let msg = "  Hello! wor_ld, example?";
    assert_eq!(None, word_bounds(msg, 0));
    assert_eq!(None, word_bounds(msg, 7));
    assert_eq!(None, word_bounds(msg, 8));
    assert_eq!(None, word_bounds(msg, 30));
    assert_eq!((2, 6), word_bounds(msg, 2).unwrap());
    assert_eq!((9, 14), word_bounds(msg, 10).unwrap());
    assert_eq!((9, 14), word_bounds(msg, 12).unwrap());
    assert_eq!((17, 23), word_bounds(msg, 18).unwrap());
}

#[test]
fn substring_test() {
    let msg = "Hello world";

    assert_eq!("o w", substring(msg, 4, 3));
    assert_eq!("o world", substring(msg, 4, 7));
    assert_eq!("Hello world", substring(msg, 0, 11));
    assert_eq!("Hello world", substring(msg, 0, 20));

    assert_eq!("o w".to_owned(), substring_owned(msg.to_owned(), 4, 3));
    assert_eq!("o world".to_owned(), substring_owned(msg.to_owned(), 4, 7));
    assert_eq!("Hello world".to_owned(), substring_owned(msg.to_owned(), 0, 11));
    assert_eq!("Hello world".to_owned(), substring_owned(msg.to_owned(), 0, 20));
}

#[test]
fn slice_test() {
    let msg = "Hello world";

    assert_eq!("o world", slice_from(msg, 4));
    assert_eq!("Hello world", slice_from(msg, 0));

    assert_eq!("Hello w", slice_to(msg, 7));
    assert_eq!("Hello world", slice_to(msg, 11));
    assert_eq!("Hello world", slice_to(msg, 20));
}
