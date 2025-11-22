use b4n_config::{SyntaxData, themes::Theme};
use b4n_tasks::highlight_all;

use crate::ui::presentation::ContentPosition;

use super::*;

fn get_styled_text(text: &str) -> Vec<StyledLine> {
    let syntax = SyntaxData::new(&Theme::default());
    let highlighter = syntax.get_highlighter("yaml").unwrap();
    let lines = text.split('\n').map(String::from).collect::<Vec<_>>();
    highlight_all(highlighter, &syntax.syntax_set, &lines).unwrap()
}

#[test]
fn sl_drain_test() {
    let styled = get_styled_text("apiVersion: v1 #with comment");

    let mut lines = styled.clone();
    lines[0].sl_drain(..5);
    assert_eq!("rsion: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(..9);
    assert_eq!("n: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(..=9);
    assert_eq!(": v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(..11);
    assert_eq!(" v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(..13);
    assert_eq!("1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(..18);
    assert_eq!("th comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(3..=5);
    assert_eq!("apision: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(3..12);
    assert_eq!("apiv1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(3..=17);
    assert_eq!("apith comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(10..=10);
    assert_eq!("apiVersion v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(10..=15);
    assert_eq!("apiVersionwith comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(10..=28);
    assert_eq!("apiVersion", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(10..=30);
    assert_eq!("apiVersion", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(10..);
    assert_eq!("apiVersion", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(11..);
    assert_eq!("apiVersion:", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(12..);
    assert_eq!("apiVersion: ", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(14..);
    assert_eq!("apiVersion: v1", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(17..);
    assert_eq!("apiVersion: v1 #w", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(30..);
    assert_eq!("apiVersion: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(100..=150);
    assert_eq!("apiVersion: v1 #with comment", lines.to_string());
}

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
    let mut styled = get_styled_text(yaml);

    styled.remove_text(Selection {
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
        styled.to_string()
    );
}
