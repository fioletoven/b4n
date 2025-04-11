use std::vec;

use crate::ui::lists::NAMESPACE;

use super::*;

#[test]
fn get_widths_test() {
    assert_eq!((0, 6, 0), Header::default().get_widths(0));
    assert_eq!((0, 6, 0), Header::default().get_widths(10));
    assert_eq!((0, 6, 0), Header::default().get_widths(15));
    assert_eq!((0, 7, 0), Header::default().get_widths(16));
    assert_eq!((0, 11, 0), Header::default().get_widths(20));
}

#[test]
fn get_full_widths_test() {
    assert_eq!((4, 6, 0), Header::default().get_full_widths(0));
    assert_eq!((4, 6, 0), Header::default().get_full_widths(10));
    assert_eq!((4, 6, 0), Header::default().get_full_widths(20));
    assert_eq!((4, 7, 0), Header::default().get_full_widths(21));
    assert_eq!((4, 11, 0), Header::default().get_full_widths(25));
    assert_eq!((4, 16, 0), Header::default().get_full_widths(30));
}

#[test]
fn get_text_name_test() {
    let test_cases = vec![
        (" NAM↑ ", 0, 5, 0),
        (" NAME↑ ", 99, 6, 0),
        (" NAME↑     ", 0, 10, 0),
        (" NAME↑ ", 0, 10, 7),
        (" NA", 0, 5, 3),
    ];

    let header = Header::default();
    for (expected, group, name, forced) in test_cases {
        assert_eq!(expected, header.get_text(ViewType::Name, group, name, forced));
    }
}

#[test]
fn get_text_compact_test() {
    let test_cases = vec![
        (" NAM↑     AGE ", 0, 5, 0),
        (" NAME↑     AGE ", 0, 6, 15),
        (" NAME↑     AGE ", 99, 6, 0),
        (" NAME↑         AGE ", 0, 10, 0),
        (" NAME↑         A", 0, 10, 16),
    ];

    let header = Header::default();
    for (expected, group, name, forced) in test_cases {
        assert_eq!(expected, header.get_text(ViewType::Compact, group, name, forced));
    }
}

#[test]
fn get_text_full_test() {
    let test_cases = vec![
        (" N/A NAM↑     AGE ", 4, 4, 0),
        (" N/A  NAME↑     AGE ", 5, 5, 0),
        (" N/A   NAME↑     AGE ", 6, 5, 0),
        (" N/A  NAME↑      AGE ", 5, 6, 0),
        (" N/A   NAME↑      AGE ", 6, 6, 0),
        (" N/A    NAME↑  ", 7, 5, 15),
    ];

    let header = Header::default();
    for (expected, group, name, forced) in test_cases {
        assert_eq!(expected, header.get_text(ViewType::Full, group, name, forced));
    }
}

#[test]
fn get_text_extra_columns_test() {
    let test_cases = vec![
        (" NAM↑ FIRST SECOND    AGE ", ViewType::Compact, 0, 5, 0),
        (" NAME↑ FIRST SECOND    AGE ", ViewType::Compact, 0, 6, 0),
        (" TEST NAME↑ FIRST SECOND    AGE ", ViewType::Full, 5, 5, 0),
        (" TEST    NAME↑    FIRST SECOND    AGE ", ViewType::Full, 8, 8, 0),
    ];

    let header = Header::from(
        Column::new("TEST"),
        Some(vec![Column::new("FIRST"), Column::new("SECOND")].into_boxed_slice()),
        Rc::new([]),
    );

    for (expected, view, group, name, forced) in test_cases {
        assert_eq!(expected, header.get_text(view, group, name, forced));
    }
}

#[test]
fn get_text_extra_columns_sized_test() {
    let test_cases = vec![
        (" NAM↑ FIRST      SECOND     AGE ", ViewType::Compact, 0, 5, 32),
        (" NAME↑ FIRST      SECOND     AGE ", ViewType::Compact, 0, 6, 33),
        (" NAMESPACE NAM↑ FIRST      SECOND     AGE ", ViewType::Full, 10, 4, 0),
        (" NAMESPACE NAME↑ FIRST      SECOND     AGE ", ViewType::Full, 10, 5, 0),
        (" NAMESPACE NAME↑    FIRST      SECOND     AGE ", ViewType::Full, 10, 8, 0),
        (" NAMESPACE   NAME↑    FIRST      SECOND     AGE ", ViewType::Full, 12, 8, 0),
        (" NAMESPACE      NAME↑", ViewType::Full, 15, 8, 21),
    ];

    let header = Header::from(
        NAMESPACE.clone(),
        Some(vec![Column::fixed("FIRST", 10, false), Column::bound("SECOND", 7, 10, false)].into_boxed_slice()),
        Rc::new([]),
    );

    for (expected, view, group, name, forced) in test_cases {
        assert_eq!(expected, header.get_text(view, group, name, forced));
    }
}

#[test]
fn get_text_extra_columns_to_right_test() {
    let test_cases = vec![
        (" NAM↑      FIRST SECOND     AGE ", ViewType::Compact, 0, 5, 0),
        (" NAME↑      FIRST SECOND     AGE ", ViewType::Compact, 0, 6, 0),
        (" NAMESPACE NAM↑      FIRST SECOND     AGE ", ViewType::Full, 10, 4, 0),
        (" NAMESPACE NAME↑      FIRST SECOND     AGE ", ViewType::Full, 10, 5, 0),
        (" NAMESPACE NAME↑         FIRST SECOND     AGE ", ViewType::Full, 10, 8, 0),
        (" NAMESPACE   NAME↑         FIRST SECOND     AGE ", ViewType::Full, 12, 8, 0),
        (" NAMESPACE      NAME↑", ViewType::Full, 15, 8, 21),
    ];

    let header = Header::from(
        NAMESPACE.clone(),
        Some(vec![Column::fixed("FIRST", 10, true), Column::bound("SECOND", 7, 10, false)].into_boxed_slice()),
        Rc::new([]),
    );

    for (expected, view, group, name, forced) in test_cases {
        assert_eq!(expected, header.get_text(view, group, name, forced));
    }
}
