use rstest::rstest;

use crate::kubernetes::resources::data;

use super::*;

#[rstest]
#[case("", "test", 0)]
#[case("tes", "test", 3)]
#[case("test  ", "test", 6)]
#[case("test           ", "test", 15)]
#[case("really long nam", "really long name", 15)]
fn get_text_name_test(#[case] expected: &str, #[case] resource: &str, #[case] terminal_width: usize) {
    let header = Header::default();
    let resource = ResourceItem::new(resource);

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Compact, terminal_width);

    assert_eq!(
        expected,
        resource.get_text(
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
    let resource = ResourceItem::new(resource);

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Compact, terminal_width);

    assert_eq!(
        expected,
        resource.get_text(
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
#[case("n/a  tes", "test", 8)]
#[case("n/a  test       ", "test", 16)]
#[case("n/a  test        n", "test", 18)]
#[case("n/a  test        n/a", "test", 20)]
#[case("n/a  test             n/a", "test", 25)]
fn get_text_full_test(#[case] expected: &str, #[case] resource: &str, #[case] terminal_width: usize) {
    let header = Header::default();
    let resource = ResourceItem::new(resource);

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Full, terminal_width);

    assert_eq!(
        expected,
        resource.get_text(
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

    let mut header = data::pod::header();
    header.set_data_length(0, 11);
    header.set_data_length(1, 39);
    header.set_data_length(2, 3);
    header.set_data_length(3, 7);
    header.set_data_length(4, 12);
    header.set_data_length(5, 11);
    header.set_data_length(6, 6);
    header.set_sort_info(2, false);

    let (namespace_width, name_width, name_extra_width) = header.get_widths(ViewType::Full, terminal_width);

    let mut resource = ResourceItem::new("local-path-provisioner-84db5d44d9-kjjp5");
    resource.namespace = Some("kube-system".to_owned());
    resource.data = Some(ResourceData {
        extra_values: vec![
            Some("5".to_owned()).into(),
            Some("1/1".to_owned()).into(),
            Some("Running".to_owned()).into(),
            Some("10.42.1.201".to_owned()).into(),
        ]
        .into_boxed_slice(),
        is_job: false,
        is_completed: false,
        is_ready: false,
        is_terminating: false,
    });

    assert_eq!(
        " NAMESPACE  NAME                                  RESTARTS↑ READY   STATUS       IP             AGE ",
        header.get_text(ViewType::Full, namespace_width, name_width, terminal_width)
    );

    assert_eq!(
        "kube-system local-path-provisioner-84db5d44d9-kjjp5       5 1/1     Running      10.42.1.201     n/a",
        resource.get_text(
            ViewType::Full,
            &header,
            terminal_width,
            namespace_width,
            name_width + name_extra_width
        )
    );
}
