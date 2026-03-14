use super::*;
use crate::filter::BasicFilterContext;

#[derive(Clone, Debug)]
pub struct TestRow {
    pub name: String,
    pub group: String,
    pub uid: String,
}

impl TestRow {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            uid: name.clone(),
            group: "default".into(),
            name,
        }
    }
}

impl Row for TestRow {
    fn uid(&self) -> &str {
        &self.uid
    }

    fn group(&self) -> &str {
        &self.group
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_name(&self, _width: usize) -> String {
        self.name.clone()
    }

    fn get_name_with_description(&self, _width: usize, desc: &str) -> String {
        format!("{} ({})", self.name, desc)
    }

    fn column_text(&self, _column: usize) -> std::borrow::Cow<'_, str> {
        self.name.as_str().into()
    }

    fn column_sort_text(&self, _column: usize) -> &str {
        &self.name
    }

    fn starts_with(&self, text: &str) -> bool {
        self.name.starts_with(text)
    }

    fn is_equal(&self, other: &str) -> bool {
        self.name == other
    }
}

impl Filterable<BasicFilterContext> for TestRow {
    fn get_context(pattern: &str, _: Option<&str>) -> BasicFilterContext {
        pattern.to_owned().into()
    }

    fn is_matching(&self, context: &mut BasicFilterContext) -> bool {
        self.name.contains(&context.pattern)
    }
}

fn make_list(names: &[&str]) -> ScrollableList<TestRow, BasicFilterContext> {
    ScrollableList::from(names.iter().map(|n| TestRow::new(*n)).collect::<Vec<_>>())
}

#[test]
fn default_is_empty() {
    let list = ScrollableList::<TestRow, BasicFilterContext>::default();
    assert!(list.is_empty());
    assert_eq!(0, list.len());
    assert_eq!(0, list.full_len());
}

#[test]
fn from_vec() {
    let list = make_list(&["a", "b", "c"]);
    assert_eq!(3, list.len());
    assert!(!list.is_empty());
}

#[test]
fn fixed_items() {
    let list = ScrollableList::<TestRow, BasicFilterContext>::fixed(vec![TestRow::new("a"), TestRow::new("b")]);
    assert_eq!(2, list.len());
    assert!(list.full_iter().all(|i| i.is_fixed));
}

#[test]
fn operations_on_empty_list() {
    let mut list = ScrollableList::<TestRow, BasicFilterContext>::default();

    list.set_dirty(true);
    list.select_all();
    list.deselect_all();
    list.invert_selection();
    list.select_highlighted_item();
    list.unhighlight_item();
    list.sort(0, false);
    list.set_filter(Some("test".into()));
    list.process_key_event(KeyCode::Down);
    list.process_scroll_up();
    list.process_scroll_down();
    list.update_page(5);
    list.remove_fixed();

    assert!(!list.is_anything_selected());
    assert!(!list.is_anything_highlighted());
    assert!(list.get_selected_items().is_empty());
    assert!(list.get_selected_uids().is_empty());
    assert_eq!(0, list.get_page().count());
    assert!(list.get_paged_names(80).is_empty());
}

#[test]
fn single_item_list() {
    let mut list = make_list(&["only"]);

    list.highlight_first_item();
    assert_eq!(Some("only"), list.get_highlighted_item_name());

    list.process_key_event(KeyCode::Down);
    assert_eq!(Some(0), list.get_highlighted_item_index());

    list.process_key_event(KeyCode::Up);
    assert_eq!(Some(0), list.get_highlighted_item_index());
}

#[test]
fn filter_reduces_len() {
    let mut list = make_list(&["abc", "bcd", "cde", "def"]);
    list.set_filter(Some("cd".into()));
    assert_eq!(2, list.len());
    assert!(list.is_filtered());
}

#[test]
fn filter_returns_false_if_same_pattern() {
    let mut list = make_list(&["abc", "bcd"]);
    assert!(list.set_filter(Some("bc".into())));
    assert!(!list.set_filter(Some("bc".into())));
}

#[test]
fn filter_none_clears_filter() {
    let mut list = make_list(&["abc", "bcd", "cde"]);
    list.set_filter(Some("bc".into()));
    assert_eq!(2, list.len());

    list.set_filter(None);
    assert_eq!(3, list.len());
    assert!(!list.is_filtered());
}

#[test]
fn filter_no_matches() {
    let mut list = make_list(&["abc", "bcd"]);
    list.set_filter(Some("xyz".into()));
    assert_eq!(0, list.len());
    assert!(list.is_empty());
}

#[test]
fn filter_deselects_all() {
    let mut list = make_list(&["abc", "bcd", "cde"]);
    list.select_all();
    assert!(list.is_anything_selected());

    list.set_filter(Some("bc".into()));
    assert!(!list.is_anything_selected());
}

#[test]
fn get_filter_returns_current() {
    let mut list = make_list(&["a"]);
    assert_eq!(None, list.filter());

    list.set_filter(Some("test".into()));
    assert_eq!(Some("test"), list.filter());
}

#[test]
fn push_respects_active_filter() {
    let mut list = make_list(&["abc", "bcd", "cde"]);
    list.set_filter(Some("bc".into()));
    assert_eq!(2, list.len());

    list.push(Item::new(TestRow::new("xbc")));
    assert_eq!(3, list.len());

    list.push(Item::new(TestRow::new("xyz")));
    assert_eq!(3, list.len());
    assert_eq!(5, list.full_len());
}

#[test]
fn update_page_basic() {
    let mut list = make_list(&["a", "b", "c", "d", "e"]);
    list.update_page(3);
    assert_eq!(3, list.page_height());
}

#[test]
fn get_page_returns_correct_slice() {
    let mut list = make_list(&["a", "b", "c", "d", "e"]);
    list.update_page(3);
    list.process_scroll_down();
    let names: Vec<_> = list.get_page().map(|i| i.data.name()).collect();
    assert_eq!(vec!["b", "c", "d"], names);
}

#[test]
fn get_page_empty_list() {
    let mut list = ScrollableList::<TestRow, BasicFilterContext>::default();
    list.update_page(5);
    assert_eq!(0, list.get_page().count());
}

#[test]
fn get_paged_names() {
    let mut list = make_list(&["a", "b", "c"]);
    list.update_page(10);
    list.highlight_first_item();

    let names = list.get_paged_names(80);
    assert_eq!(3, names.len());
    assert!(names[0].1);
    assert!(!names[1].1);
    assert!(!names[2].1);
}

#[test]
fn get_paged_names_with_description() {
    let mut list = make_list(&["a", "b"]);
    list.update_page(10);
    list.highlight_first_item();

    let names = list.get_paged_names_with_description(80, "desc");
    assert_eq!("a (desc)", names[0].0);
    assert!(names[0].1);
}
