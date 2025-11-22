use crate::ui::presentation::ContentPosition;

use super::*;

#[test]
fn remove_text_test() {
    let yaml = r#"apiVersion: v1
kind: Pod
metadata:
  creationTimestamp: 2025-08-27T19:31:08Z
  generateName: coredns-6799fbcd5-
  labels:
    k8s-app: kube-dns
    pod-template-hash: 6799fbcd5
  name: coredns-6799fbcd5-pt4xz
  namespace: kube-system"#;

    let mut lines = yaml.split('\n').map(String::from).collect::<Vec<_>>();
    lines.remove_text(Selection {
        start: ContentPosition::new(8, 3),
        end: ContentPosition::new(5, 5),
    });

    assert_eq!(
        r#"apiVersion: v1
kind: Pod
metadata:
  creatils:
    k8s-app: kube-dns
    pod-template-hash: 6799fbcd5
  name: coredns-6799fbcd5-pt4xz
  namespace: kube-system"#,
        lines.join("\n")
    );
}

#[test]
fn remove_text_one_line_test() {
    let mut text = vec!["Some Test_Line".to_owned()];

    text.remove_text(Selection {
        start: ContentPosition::new(6, 0),
        end: ContentPosition::new(8, 0),
    });

    assert_eq!("Some T_Line", text[0])
}
