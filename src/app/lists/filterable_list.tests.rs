use super::*;

#[test]
fn len_test() {
    let mut list = FilterableList::from(vec![1, 2, 3, 4, 5]);
    assert_eq!(5, list.len());

    list.filter(|i| *i > 3);
    assert_eq!(2, list.len());
}

#[test]
fn iterators_test() {
    let mut list = FilterableList::from(vec!["abc", "bcd", "cde"]);

    let mut iter = list.iter();
    assert_eq!(Some(&"abc"), iter.next());
    assert_eq!(Some(&"bcd"), iter.next());
    assert_eq!(Some(&"cde"), iter.next());
    assert_eq!(None, iter.next());

    list.filter(|i| i.contains("bc"));

    let mut iter = list.iter();
    assert_eq!(Some(&"abc"), iter.next());
    assert_eq!(Some(&"bcd"), iter.next());
    assert_eq!(None, iter.next());

    let mut iter = list.full_iter();
    assert_eq!(Some(&"abc"), iter.next());
    assert_eq!(Some(&"bcd"), iter.next());
    assert_eq!(Some(&"cde"), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn mutable_iterators_test() {
    let mut list = FilterableList::from(vec!["abc", "bcd", "cde"]);

    list.filter(|i| i.contains("bc"));

    for i in &mut list {
        *i = "test";
    }

    list.filter_reset();

    assert_eq!("test", list[0]);
    assert_eq!("test", list[1]);
    assert_eq!("cde", list[2]);
}
