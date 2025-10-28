use crate::filter::filterable_list_tests::TestItem;

use super::*;

impl Row for TestItem {
    fn uid(&self) -> &str {
        todo!()
    }

    fn group(&self) -> &str {
        todo!()
    }

    fn name(&self) -> &str {
        todo!()
    }

    fn get_name(&self, _width: usize) -> String {
        todo!()
    }

    fn column_text(&self, _column: usize) -> std::borrow::Cow<'_, str> {
        todo!()
    }

    fn column_sort_text(&self, _column: usize) -> &str {
        todo!()
    }
}

#[test]
fn filter_test() {
    let mut list = ScrollableList::from([1, 2, 3, 4, 5, 10, 11].iter().map(TestItem::new).collect::<Vec<_>>());
    assert_eq!(7, list.len());

    list.filter(Some("1".to_string()));
    assert_eq!(3, list.len());

    list.push(TestItem::new("12"));
    assert_eq!(4, list.len());

    list.push(TestItem::new("23"));
    assert_eq!(4, list.len());

    list.push(TestItem::new("13"));
    assert_eq!(5, list.len());

    list.filter(Some("2".to_string()));
    assert_eq!(3, list.len());

    list.filter(None);
    assert_eq!(10, list.len());
}
