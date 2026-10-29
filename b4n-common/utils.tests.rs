use super::*;

#[test]
fn word_bounds_test() {
    let msg = "Hello";
    assert_eq!((0, 4), word_bounds(msg, 2).unwrap());

    let msg = "Hello world";
    assert_eq!((0, 4), word_bounds(msg, 2).unwrap());
    assert_eq!((6, 10), word_bounds(msg, 8).unwrap());

    let msg = " Hello world";
    assert_eq!((1, 5), word_bounds(msg, 3).unwrap());

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

#[test]
fn sanitize_and_split_test() {
    assert_eq!(vec!["", ""], sanitize_and_split("\n"));
    assert_eq!(vec!["", "test"], sanitize_and_split("\ntest"));
    assert_eq!(vec!["test", ""], sanitize_and_split("test\n"));
    assert_eq!(vec!["test1", "test2"], sanitize_and_split("test1\ntest2"));
    assert_eq!(vec!["test1", "test2"], sanitize_and_split("test1\r\ntest2"));
}

#[test]
fn display_clean_test() {
    for path in [
        "",
        ".",
        "logs/output.txt",
        r"C:\logs\output.txt",
        r"\\server\share\output.txt",
    ] {
        assert_eq!(
            Path::new(path).display().to_string(),
            Path::new(path).display_clean().to_string()
        );
    }
}

#[cfg(windows)]
#[test]
fn display_clean_windows_test() {
    use std::os::windows::ffi::OsStringExt;

    assert_eq!(
        r"C:\logs\output.txt",
        Path::new(r"\\?\C:\logs\output.txt").display_clean().to_string()
    );
    assert_eq!(
        r"\\server\share\output.txt",
        Path::new(r"\\?\UNC\server\share\output.txt").display_clean().to_string()
    );
    assert_eq!(
        "C:\\logs\\\u{fffd}.txt",
        Path::new(&std::ffi::OsString::from_wide(&[
            92, 92, 63, 92, 67, 58, 92, 108, 111, 103, 115, 92, 0xD800, 46, 116, 120, 116,
        ]))
        .display_clean()
        .to_string()
    );
}

#[cfg(not(windows))]
#[test]
fn display_clean_unix_test() {
    let path = Path::new(r"\\?\C:\logs\output.txt");
    assert_eq!(path.display().to_string(), path.display_clean().to_string());
}
