use super::*;

#[test]
fn kind_with_group_test() {
    let kind: Kind = "pod".into();
    assert_eq!("pod", kind.name());
    assert!(!kind.has_group());
    assert_eq!("", kind.group());
    assert!(!kind.has_version());
    assert_eq!("", kind.version());

    let kind: Kind = "pod.non_core".into();
    assert_eq!("pod", kind.name());
    assert!(kind.has_group());
    assert_eq!("non_core", kind.group());
    assert!(!kind.has_version());
    assert_eq!("", kind.version());

    let kind: Kind = "pod.non_core/v1".into();
    assert_eq!("pod", kind.name());
    assert!(kind.has_group());
    assert_eq!("non_core", kind.group());
    assert!(kind.has_version());
    assert_eq!("v1", kind.version());

    let kind: Kind = "pod./v1".into();
    assert_eq!("pod", kind.name());
    assert!(!kind.has_group());
    assert_eq!("", kind.group());
    assert!(!kind.has_version());
    assert_eq!("", kind.version());
}
