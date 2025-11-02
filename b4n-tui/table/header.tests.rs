use std::vec;

use super::*;

#[test]
fn get_widths_test() {
    assert_eq!((0, 6, 0), Header::default().get_compact_widths(0));
    assert_eq!((0, 6, 0), Header::default().get_compact_widths(10));
    assert_eq!((0, 6, 0), Header::default().get_compact_widths(15));
    assert_eq!((0, 7, 0), Header::default().get_compact_widths(16));
    assert_eq!((0, 11, 0), Header::default().get_compact_widths(20));
}

#[test]
fn get_full_widths_test() {
    assert_eq!((11, 6, 0), Header::default().get_full_widths(0));
    assert_eq!((11, 6, 0), Header::default().get_full_widths(10));
    assert_eq!((11, 6, 0), Header::default().get_full_widths(27));
    assert_eq!((11, 7, 0), Header::default().get_full_widths(28));
    assert_eq!((11, 9, 0), Header::default().get_full_widths(30));
    assert_eq!((11, 14, 0), Header::default().get_full_widths(35));
}

#[test]
fn get_text_name_test() {
    let test_cases = vec![
        (" NA", 3),
        (" NAME↑ ", 0),
        (" NAME↑ ", 7),
        (" NAME↑     ", 11),
        (" NAME↑         ", 15),
    ];

    let mut header = Header::default();
    for (expected, width) in test_cases {
        assert_eq!(expected, header.get_text(ViewType::Name, width));
    }
}

#[test]
fn get_text_compact_test() {
    let test_cases = vec![
        (" NAME", 5),
        (" NAME↑ ", 7),
        (" NAME↑   ", 9),
        (" NAME↑     A", 12),
        (" NAME↑     AGE ", 0),
        (" NAME↑     AGE ", 15),
        (" NAME↑         AGE ", 19),
    ];

    let mut header = Header::default();
    for (expected, width) in test_cases {
        assert_eq!(expected, header.get_text(ViewType::Compact, width));
    }
}

#[test]
fn get_text_full_test() {
    let test_cases = vec![
        (" NAMESPA", 8),
        (" NAMESPACE  NAM", 15),
        (" NAMESPACE  NAME↑     ", 22),
        (" NAMESPACE  NAME↑      AGE ", 0),
        (" NAMESPACE  NAME↑      AGE ", 27),
        (" NAMESPACE  NAME↑        AGE ", 29),
    ];

    let mut header = Header::default();
    for (expected, width) in test_cases {
        assert_eq!(expected, header.get_text(ViewType::Full, width));
    }
}

#[test]
fn get_text_extra_columns_test() {
    let test_cases = vec![
        (" NAME↑ ", ViewType::Name, 0),
        (" NAME↑    ", ViewType::Name, 10),
        (" NAME↑ FIRST SECOND    AGE ", ViewType::Compact, 0),
        (" TEST NAME↑  FIRST SECOND    AGE ", ViewType::Full, 0),
        (" TEST NAME↑       FIRST SECOND    AGE ", ViewType::Full, 38),
    ];

    let mut header = Header::from(
        Column::new("TEST"),
        Some(vec![Column::new("FIRST"), Column::new("SECOND")].into_boxed_slice()),
        Rc::new([]),
    );

    for (expected, view, width) in test_cases {
        assert_eq!(expected, header.get_text(view, width));
    }
}

#[test]
fn get_text_extra_columns_sized_test() {
    let test_cases = vec![
        (" NAME↑ FIRST      SEC", ViewType::Compact, 21),
        (" NAME↑ FIRST      SECOND     AGE ", ViewType::Compact, 33),
        (" NAMESPACE  NAME↑  FI", ViewType::Full, 21),
        (" NAMESPACE  NAME↑  FIRST      SECOND     AGE ", ViewType::Full, 0),
        (" NAMESPACE  NAME↑    FIRST      SECOND     AGE ", ViewType::Full, 47),
        (" NAMESPACE  NAME↑            FIRST      SECOND     AGE ", ViewType::Full, 55),
    ];

    let mut header = Header::from(
        NAMESPACE,
        Some(vec![Column::fixed("FIRST", 10, false), Column::bound("SECOND", 7, 10, false)].into_boxed_slice()),
        Rc::new([]),
    );

    for (expected, view, width) in test_cases {
        assert_eq!(expected, header.get_text(view, width));
    }
}

#[test]
fn get_text_extra_columns_to_right_test() {
    let test_cases = vec![
        (" NAME↑      FIRST SE", ViewType::Compact, 20),
        (" NAME↑      FIRST SECOND     AGE ", ViewType::Compact, 0),
        (" NAME↑        FIRST SECOND     AGE ", ViewType::Compact, 35),
        (" NAMESPACE  NAM", ViewType::Full, 15),
        (" NAMESPACE  NAME↑    ", ViewType::Full, 21),
        (" NAMESPACE  NAME↑       FIRST SECOND     AGE ", ViewType::Full, 0),
        (" NAMESPACE  NAME↑       FIRST SECOND     AGE ", ViewType::Full, 45),
        (" NAMESPACE  NAME↑                 FIRST SECOND     AGE ", ViewType::Full, 55),
    ];

    let mut header = Header::from(
        NAMESPACE,
        Some(vec![Column::fixed("FIRST", 10, true), Column::bound("SECOND", 7, 10, false)].into_boxed_slice()),
        Rc::new([]),
    );

    for (expected, view, width) in test_cases {
        assert_eq!(expected, header.get_text(view, width));
    }
}
