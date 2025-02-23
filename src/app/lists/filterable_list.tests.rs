use crate::{app::lists::Row, kubernetes::resources::Kind};

use super::*;

#[test]
fn len_test() {
    let mut list = FilterableList::from(
        vec![1, 2, 3, 4, 5, 10, 11]
            .iter()
            .map(|i| Kind::new(String::new(), i.to_string(), i.to_string()))
            .collect::<Vec<_>>(),
    );
    assert_eq!(7, list.len());

    let mut context = Kind::get_context("1", None);
    list.filter(&mut context);
    assert_eq!(3, list.len());
}

#[test]
fn iterators_test() {
    let mut list = FilterableList::from(
        vec!["abc", "bcd", "cde"]
            .iter()
            .map(|i| Kind::new(String::new(), i.to_string(), i.to_string()))
            .collect::<Vec<_>>(),
    );

    let mut iter = list.iter();
    assert_eq!("abc", iter.next().unwrap().name());
    assert_eq!("bcd", iter.next().unwrap().name());
    assert_eq!("cde", iter.next().unwrap().name());
    assert!(iter.next().is_none());

    let mut context = Kind::get_context("bc", None);
    list.filter(&mut context);

    let mut iter = list.iter();
    assert_eq!("abc", iter.next().unwrap().name());
    assert_eq!("bcd", iter.next().unwrap().name());
    assert!(iter.next().is_none());

    let mut iter = list.full_iter();
    assert_eq!("abc", iter.next().unwrap().name());
    assert_eq!("bcd", iter.next().unwrap().name());
    assert_eq!("cde", iter.next().unwrap().name());
    assert!(iter.next().is_none());
}

#[test]
fn mutable_iterators_test() {
    let mut list = FilterableList::from(
        vec!["abc", "bcd", "cde"]
            .iter()
            .map(|i| Kind::new(String::new(), i.to_string(), i.to_string()))
            .collect::<Vec<_>>(),
    );

    let mut context = Kind::get_context("bc", None);
    list.filter(&mut context);

    for i in &mut list {
        *i = Kind::new(String::new(), "test".to_string(), "test_v".to_string());
    }

    list.filter_reset();

    assert_eq!("test", list[0].name());
    assert_eq!("test", list[1].name());
    assert_eq!("cde", list[2].name());
}
