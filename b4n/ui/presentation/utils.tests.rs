use crate::ui::presentation::ContentPosition;

use super::*;

#[test]
fn remove_text_test() {
    let yaml = r"apiVersion: v1
kind: Pod
metadata:
  creationTimestamp: 2025-08-27T19:31:08Z
  generateName: coredns-6799fbcd5-
  labels:
    k8s-app: kube-dns
    pod-template-hash: 6799fbcd5
  name: coredns-6799fbcd5-pt4xz
  namespace: kube-system";

    let mut lines = yaml.split('\n').map(String::from).collect::<Vec<_>>();
    let removed = lines.remove_text(&Selection {
        start: ContentPosition::new(8, 3),
        end: ContentPosition::new(5, 5),
    });

    assert_eq!(
        r"apiVersion: v1
kind: Pod
metadata:
  creatils:
    k8s-app: kube-dns
    pod-template-hash: 6799fbcd5
  name: coredns-6799fbcd5-pt4xz
  namespace: kube-system",
        lines.join("\n")
    );

    assert_eq!(
        r"onTimestamp: 2025-08-27T19:31:08Z
  generateName: coredns-6799fbcd5-
  labe",
        removed.join("\n")
    );
}

#[test]
fn remove_text_one_line_test() {
    let mut text = vec!["Some Test_Line".to_owned()];

    let removed = text.remove_text(&Selection {
        start: ContentPosition::new(6, 0),
        end: ContentPosition::new(8, 0),
    });

    assert_eq!("Some T_Line", text[0]);
    assert_eq!("est", removed[0]);
}

#[test]
fn remove_text_line_end_test() {
    let mut text = vec!["first line".to_owned(), "second line".to_owned()];

    let removed = text.remove_text(&Selection {
        start: ContentPosition::new(10, 0),
        end: ContentPosition::new(10, 0),
    });

    assert_eq!("first linesecond line", text[0]);
    assert_eq!("", removed[0]);
}

#[test]
fn insert_text_test() {
    let yaml = r"apiVersion: v1
kind: Pod
metadata:
  creationTimestamp: 2025-08-27T19:31:08Z
  generateName: coredns-6799fbcd5-";

    let to_insert = r"_lines
to insert
into the yaml_";

    let text = to_insert.split('\n').map(String::from).collect::<Vec<_>>();
    let mut actual = yaml.split('\n').map(String::from).collect::<Vec<_>>();
    actual.insert_text(ContentPosition::new(5, 3), text);

    let expected = r"apiVersion: v1
kind: Pod
metadata:
  cre_lines
to insert
into the yaml_ationTimestamp: 2025-08-27T19:31:08Z
  generateName: coredns-6799fbcd5-";

    assert_eq!(expected, actual.join("\n"));
}
