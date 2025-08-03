use std::rc::Rc;

use crate::{
    kubernetes::resources::{ResourceData, ResourceItem},
    ui::lists::{Column, NAMESPACE},
};
use rstest::rstest;

use super::*;

#[rstest]
#[case("", "test", 0)]
#[case("tes", "test", 3)]
#[case("test  ", "test", 6)]
#[case("test           ", "test", 15)]
#[case("really long nam", "really long name", 15)]
fn get_text_name_test(#[case] expected: &str, #[case] resource: &str, #[case] terminal_width: usize) {
    let header = Header::default();
    let item = Item::new(ResourceItem::new(resource));

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Compact, terminal_width);

    assert_eq!(
        expected,
        item.get_text(
            ViewType::Name,
            &header,
            terminal_width,
            namespace_width,
            name_width + name_extra_width
        )
    );
}

#[rstest]
#[case("", "test", 0)]
#[case("test ", "test", 5)]
#[case("test        n/", "test", 14)]
#[case("test        n/a", "test", 15)]
#[case("test         n/a", "test", 16)]
fn get_text_compact_test(#[case] expected: &str, #[case] resource: &str, #[case] terminal_width: usize) {
    let header = Header::default();
    let item = Item::new(ResourceItem::new(resource));

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Compact, terminal_width);

    assert_eq!(
        expected,
        item.get_text(
            ViewType::Compact,
            &header,
            terminal_width,
            namespace_width,
            name_width + name_extra_width
        )
    );
}

#[rstest]
#[case("", "test", 0)]
#[case("n/a  ", "test", 5)]
#[case("n/a     ", "test", 8)]
#[case("n/a         tes", "test", 15)]
#[case("n/a         test       ", "test", 23)]
#[case("n/a         test        n", "test", 25)]
#[case("n/a         test        n/a", "test", 27)]
#[case("n/a         test             n/a", "test", 32)]
fn get_text_full_test(#[case] expected: &str, #[case] resource: &str, #[case] terminal_width: usize) {
    let header = Header::default();
    let item = Item::new(ResourceItem::new(resource));

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Full, terminal_width);

    assert_eq!(
        expected,
        item.get_text(
            ViewType::Full,
            &header,
            terminal_width,
            namespace_width,
            name_width + name_extra_width
        )
    );
}

#[test]
fn get_text_pod_test() {
    // " NAMESPACE  NAME                                  RESTARTS↑ READY   STATUS       IP             AGE "
    // "kube-system local-path-provisioner-84db5d44d9-kjjp5       5 1/1     Running      10.42.1.201     n/a"

    let terminal_width = 100;

    let mut header = crate::kubernetes::resources::pod::header();
    header.set_data_length(0, 11);
    header.set_data_length(1, 39);
    header.set_data_length(2, 3);
    header.set_data_length(3, 7);
    header.set_data_length(4, 12);
    header.set_data_length(5, 11);
    header.set_data_length(6, 6);
    header.set_sort_info(2, false);

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Full, terminal_width);

    let mut item = Item::new(ResourceItem::new("local-path-provisioner-84db5d44d9-kjjp5"));
    item.data.namespace = Some("kube-system".to_owned());
    item.data.data = Some(ResourceData {
        extra_values: vec![
            Some("5".to_owned()).into(),
            Some("1/1".to_owned()).into(),
            Some("Running".to_owned()).into(),
            Some("10.42.1.201".to_owned()).into(),
        ]
        .into_boxed_slice(),
        ..Default::default()
    });

    assert_eq!(
        " NAMESPACE  NAME                                  RESTARTS↑ READY   STATUS       IP             AGE ",
        header.get_text(ViewType::Full, terminal_width)
    );

    assert_eq!(
        "kube-system local-path-provisioner-84db5d44d9-kjjp5       5 1/1     Running      10.42.1.201     n/a",
        item.get_text(
            ViewType::Full,
            &header,
            terminal_width,
            namespace_width,
            name_width + name_extra_width
        )
    );
}

#[test]
fn align_column_to_right_test() {
    // " NAMESPACE  NAME                                 RESTARTS↑          IP    AGE "
    // "kube-system local-path-provisioner-84db5d44d9-kjjp5 555555 10.42.1.201     n/a"

    let terminal_width = 78;

    let mut header = Header::from(
        NAMESPACE,
        Some(Box::new([
            Column::bound("RESTARTS", 5, 10, true),
            Column::bound("IP", 11, 16, true),
        ])),
        Rc::new([' ', 'N', 'R', 'I', 'A']),
    );

    header.set_data_length(0, 11);
    header.set_data_length(1, 39);
    header.set_data_length(2, 6);
    header.set_data_length(3, 11);
    header.set_data_length(4, 6);
    header.set_sort_info(2, false);

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Full, terminal_width);

    let mut item = Item::new(ResourceItem::new("local-path-provisioner-84db5d44d9-kjjp5"));
    item.data.namespace = Some("kube-system".to_owned());
    item.data.data = Some(ResourceData {
        extra_values: vec![Some("555555".to_owned()).into(), Some("10.42.1.201".to_owned()).into()].into_boxed_slice(),
        ..Default::default()
    });

    assert_eq!(
        " NAMESPACE  NAME                                 RESTARTS↑          IP    AGE ",
        header.get_text(ViewType::Full, terminal_width)
    );

    assert_eq!(
        "kube-system local-path-provisioner-84db5d44d9-kjjp5 555555 10.42.1.201     n/a",
        item.get_text(
            ViewType::Full,
            &header,
            terminal_width,
            namespace_width,
            name_width + name_extra_width
        )
    );
}
