use crate::filter::BasicFilterContext;

use super::*;

pub struct TestItem {
    pub name: String,
}

impl TestItem {
    pub fn new(name: impl std::fmt::Display) -> Self {
        Self { name: name.to_string() }
    }
}

impl Filterable<BasicFilterContext> for TestItem {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.name.contains(&context.pattern)
    }
}

#[test]
fn len_test() {
    let mut list = FilterableList::from([1, 2, 3, 4, 5, 10, 11].iter().map(TestItem::new).collect::<Vec<_>>());
    assert_eq!(7, list.len());

    let mut context = TestItem::get_context("1", None);
    list.filter(&mut context);
    assert_eq!(3, list.len());
}

#[test]
fn iterators_test() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut iter = list.iter();
    assert_eq!("abc", iter.next().unwrap().name);
    assert_eq!("bcd", iter.next().unwrap().name);
    assert_eq!("cde", iter.next().unwrap().name);
    assert!(iter.next().is_none());

    let mut context = TestItem::get_context("bc", None);
    list.filter(&mut context);

    let mut iter = list.iter();
    assert_eq!("abc", iter.next().unwrap().name);
    assert_eq!("bcd", iter.next().unwrap().name);
    assert!(iter.next().is_none());

    let mut iter = list.full_iter();
    assert_eq!("abc", iter.next().unwrap().name);
    assert_eq!("bcd", iter.next().unwrap().name);
    assert_eq!("cde", iter.next().unwrap().name);
    assert!(iter.next().is_none());
}

#[test]
fn mutable_iterators_test() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("bc", None);
    list.filter(&mut context);

    for i in &mut list {
        *i = TestItem::new("test");
    }

    list.filter_reset();

    assert_eq!("test", list[0].name);
    assert_eq!("test", list[1].name);
    assert_eq!("cde", list[2].name);
}
