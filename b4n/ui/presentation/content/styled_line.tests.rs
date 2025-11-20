use b4n_config::{SyntaxData, themes::Theme};
use b4n_tasks::highlight_all;

use crate::ui::presentation::ContentPosition;

use super::*;

fn get_styled_text(text: &str) -> Vec<StyledLine> {
    let syntax = SyntaxData::new(&Theme::default());
    let highlighter = syntax.get_highlighter("yaml");
    let lines = text.split('\n').map(String::from).collect::<Vec<_>>();
    highlight_all(highlighter, &syntax.syntax_set, &lines).unwrap()
}

#[test]
fn sl_drain_to_test() {
    let styled = get_styled_text("apiVersion: v1 #with comment");

    let mut lines = styled.clone();
    lines[0].sl_drain_to(5);
    assert_eq!("rsion: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain_to(10);
    assert_eq!(": v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain_to(11);
    assert_eq!(" v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain_to(13);
    assert_eq!("1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain_to(18);
    assert_eq!("th comment", lines.to_string());
}

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
    let mut styled = get_styled_text(yaml);

    styled.remove_text(Selection {
        start: ContentPosition::new(8, 3),
        end: ContentPosition::new(5, 5),
    });

    assert_eq!(
        r#"apiVersion: v1
kind: Pod
metadata:
  creatiels:
    k8s-app: kube-dns
    pod-template-hash: 6799fbcd5
  name: coredns-6799fbcd5-pt4xz
  namespace: kube-system"#,
        styled.to_string()
    );
}
