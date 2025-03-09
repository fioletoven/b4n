use super::*;

#[test]
fn push_column_test() {
    let test_cases = vec![
        ("", 0, false, false),
        ("TEST", 4, false, false),
        ("TEST ", 5, false, false),
        ("TES↑", 4, true, false),
        ("TEST↑", 5, true, false),
        ("TEST↑ ", 6, true, false),
        ("TEST↑     ", 10, true, false),
        ("TEST↓ ", 6, true, true),
        ("TEST↓     ", 10, true, true),
    ];

    let mut column = Column::new("TEST");
    let mut actual = String::new();

    for (expected, len, is_sorted, is_descending) in test_cases {
        column.is_sorted = is_sorted;

        actual.clear();
        actual.push_column(&column, len, is_descending);

        assert_eq!(expected, actual);
    }
}

#[test]
fn non_breaking_spaces_test() {
    let column = Column::new("COLUMN WITH SPACES");
    let mut actual = String::new();
    actual.push_column(&column, 20, false);

    assert_eq!("COLUMN WITH SPACES  ", actual);
}
