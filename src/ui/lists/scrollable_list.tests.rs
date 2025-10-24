use crate::kubernetes::kinds::KindItem;

use super::*;

#[test]
fn filter_test() {
    let mut list = ScrollableList::from(
        [1, 2, 3, 4, 5, 10, 11]
            .iter()
            .map(|i| KindItem::new("", i.to_string(), &(i.to_string())))
            .collect::<Vec<_>>(),
    );
    assert_eq!(7, list.len());

    list.filter(Some("1".to_string()));
    assert_eq!(3, list.len());

    list.push(KindItem::new("", "12".to_string(), "12"));
    assert_eq!(4, list.len());

    list.push(KindItem::new("", "23".to_string(), "23"));
    assert_eq!(4, list.len());

    list.push(KindItem::new("", "13".to_string(), "13"));
    assert_eq!(5, list.len());

    list.filter(Some("2".to_string()));
    assert_eq!(3, list.len());

    list.filter(None);
    assert_eq!(10, list.len());
}
