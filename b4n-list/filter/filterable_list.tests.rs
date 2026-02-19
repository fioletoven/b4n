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

#[test]
fn empty_list() {
    let mut list = FilterableList::<TestItem, BasicFilterContext>::from(vec![]);
    assert_eq!(0, list.len());
    assert!(list.is_empty());
    assert!(list.iter().next().is_none());
    assert!((&mut list).into_iter().next().is_none());

    let mut context = TestItem::get_context("anything", None);
    list.filter(&mut context);
    assert_eq!(0, list.len());
    assert!(list.iter().next().is_none());
}

#[test]
fn filter_matches_nothing() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("xyz", None);
    list.filter(&mut context);

    assert_eq!(0, list.len());
    assert!(list.is_empty());
    assert!(list.iter().next().is_none());
    assert!((&mut list).into_iter().next().is_none());

    assert_eq!(3, list.full_iter().count());
}

#[test]
fn filter_matches_everything() {
    let mut list = FilterableList::from(["abc", "abcd", "abcde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("abc", None);
    list.filter(&mut context);

    assert_eq!(3, list.len());
    assert_eq!(3, list.iter().count());
}

#[test]
fn filter_then_reset() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("bc", None);
    list.filter(&mut context);
    assert_eq!(2, list.len());

    list.filter_reset();
    assert_eq!(3, list.len());
    assert_eq!("abc", list.iter().next().unwrap().name);
}

#[test]
fn refilter_with_different_pattern() {
    let mut list = FilterableList::from(["abc", "bcd", "cde", "def"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("bc", None);
    list.filter(&mut context);
    assert_eq!(2, list.len());
    let names: Vec<_> = list.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(vec!["abc", "bcd"], names);

    let mut context = TestItem::get_context("de", None);
    list.filter(&mut context);
    assert_eq!(2, list.len());
    let names: Vec<_> = list.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(vec!["cde", "def"], names);
}

#[test]
fn single_element_list() {
    let mut list = FilterableList::from(vec![TestItem::new("only")]);

    assert_eq!(1, list.len());
    assert_eq!("only", list.iter().next().unwrap().name);

    let mut context = TestItem::get_context("only", None);
    list.filter(&mut context);
    assert_eq!(1, list.len());

    let mut context = TestItem::get_context("nope", None);
    list.filter(&mut context);
    assert_eq!(0, list.len());
}

#[test]
fn index_access_unfiltered() {
    let list = FilterableList::from(["a", "b", "c"].iter().map(TestItem::new).collect::<Vec<_>>());

    assert_eq!("a", list[0].name);
    assert_eq!("b", list[1].name);
    assert_eq!("c", list[2].name);
}

#[test]
fn index_access_filtered() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("cd", None);
    list.filter(&mut context);

    assert_eq!("bcd", list[0].name);
    assert_eq!("cde", list[1].name);
}

#[test]
#[should_panic]
fn index_out_of_bounds_unfiltered() {
    let list = FilterableList::from(vec![TestItem::new("a")]);
    let _ = &list[1];
}

#[test]
#[should_panic]
fn index_out_of_bounds_filtered() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("bc", None);
    list.filter(&mut context);

    let _ = &list[2];
}

#[test]
fn mutable_iterator_unfiltered() {
    let mut list = FilterableList::from(["a", "b", "c"].iter().map(TestItem::new).collect::<Vec<_>>());

    for item in &mut list {
        item.name = item.name.to_uppercase();
    }

    let names: Vec<_> = list.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(vec!["A", "B", "C"], names);
}

#[test]
fn mutable_iterator_on_empty_filtered() {
    let mut list = FilterableList::from(["abc", "bcd"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("xyz", None);
    list.filter(&mut context);

    for item in &mut list {
        item.name = "should not happen".into();
    }

    list.filter_reset();
    assert_eq!("abc", list[0].name);
    assert_eq!("bcd", list[1].name);
}

#[test]
fn full_iter_unaffected_by_filter() {
    let mut list = FilterableList::from(["abc", "bcd", "cde", "def"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("bc", None);
    list.filter(&mut context);

    assert_eq!(2, list.iter().count());
    assert_eq!(4, list.full_iter().count());

    let full_names: Vec<_> = list.full_iter().map(|i| i.name.as_str()).collect();
    assert_eq!(vec!["abc", "bcd", "cde", "def"], full_names);
}

#[test]
fn iterator_collect() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    let mut context = TestItem::get_context("c", None);
    list.filter(&mut context);

    let names: Vec<String> = list.iter().map(|i| i.name.clone()).collect();
    assert_eq!(vec!["abc", "bcd", "cde"], names);

    let mut context = TestItem::get_context("cd", None);
    list.filter(&mut context);

    let names: Vec<String> = list.iter().map(|i| i.name.clone()).collect();
    assert_eq!(vec!["bcd", "cde"], names);
}

#[test]
fn filter_preserves_order() {
    let mut list = FilterableList::from(
        ["z_a", "a_z", "z_b", "b_z", "z_c"]
            .iter()
            .map(TestItem::new)
            .collect::<Vec<_>>(),
    );

    let mut context = TestItem::get_context("z_", None);
    list.filter(&mut context);

    let names: Vec<_> = list.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(vec!["z_a", "z_b", "z_c"], names);
}

#[test]
fn multiple_filter_reset_cycles() {
    let mut list = FilterableList::from(["abc", "bcd", "cde"].iter().map(TestItem::new).collect::<Vec<_>>());

    for _ in 0..3 {
        let mut context = TestItem::get_context("bc", None);
        list.filter(&mut context);
        assert_eq!(2, list.len());

        list.filter_reset();
        assert_eq!(3, list.len());
    }
}

#[test]
fn len_and_is_empty_consistency() {
    let mut list = FilterableList::from(["abc", "bcd"].iter().map(TestItem::new).collect::<Vec<_>>());

    assert!(!list.is_empty());
    assert_eq!(2, list.len());

    let mut context = TestItem::get_context("abc", None);
    list.filter(&mut context);
    assert!(!list.is_empty());
    assert_eq!(1, list.len());

    let mut context = TestItem::get_context("xyz", None);
    list.filter(&mut context);
    assert!(list.is_empty());
    assert_eq!(0, list.len());
}
