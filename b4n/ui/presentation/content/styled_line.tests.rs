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
fn char_to_index_test() {
    let styled = get_styled_text("apiVersiąn: v1 #with comment");
    assert_eq!(Some(5), styled[0].char_to_index(5));
    assert_eq!(Some(19), styled[0].char_to_index(18));
    assert_eq!(Some(28), styled[0].char_to_index(27));
    assert_eq!(None, styled[0].char_to_index(28));
}

#[test]
fn char_boundaries_test() {
    let styled = get_styled_text("  ąęśćńół: test");

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(0), Some(6));
    assert_eq!("ńół: test", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(1), Some(7));
    assert_eq!(" ół: test", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(2), Some(7));
    assert_eq!("  ół: test", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(2), Some(8));
    assert_eq!("  ł: test", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(2), Some(9));
    assert_eq!("  : test", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(2), Some(10));
    assert_eq!("   test", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(2), Some(11));
    assert_eq!("  test", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(2), Some(12));
    assert_eq!("  est", lines.to_string());
}

#[test]
fn sl_drain_test() {
    let styled = get_styled_text("apiVersion: v1 #with comment");

    let mut lines = styled.clone();
    lines[0].sl_drain(None, Some(5));
    assert_eq!("rsion: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(None, Some(9));
    assert_eq!("n: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(None, Some(10));
    assert_eq!(": v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(None, Some(11));
    assert_eq!(" v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(None, Some(13));
    assert_eq!("1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(None, Some(18));
    assert_eq!("th comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(3), Some(6));
    assert_eq!("apision: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(3), Some(12));
    assert_eq!("apiv1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(3), Some(18));
    assert_eq!("apith comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(10), Some(11));
    assert_eq!("apiVersion v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(10), Some(16));
    assert_eq!("apiVersionwith comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(10), Some(29));
    assert_eq!("apiVersion", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(10), Some(30));
    assert_eq!("apiVersion", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(10), None);
    assert_eq!("apiVersion", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(11), None);
    assert_eq!("apiVersion:", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(12), None);
    assert_eq!("apiVersion: ", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(14), None);
    assert_eq!("apiVersion: v1", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(17), None);
    assert_eq!("apiVersion: v1 #w", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(30), None);
    assert_eq!("apiVersion: v1 #with comment", lines.to_string());

    let mut lines = styled.clone();
    lines[0].sl_drain(Some(100), Some(150));
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

    styled.remove_text(&Selection {
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
