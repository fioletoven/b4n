use b4n_config::{SyntaxData, themes::Theme};
use b4n_tasks::highlight_all;
use rstest::rstest;

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

#[rstest]
#[case(Some(0), Some(6), "ńół: test")]
#[case(Some(1), Some(7), " ół: test")]
#[case(Some(2), Some(7), "  ół: test")]
#[case(Some(2), Some(8), "  ł: test")]
#[case(Some(2), Some(9), "  : test")]
#[case(Some(2), Some(10), "   test")]
#[case(Some(2), Some(11), "  test")]
#[case(Some(2), Some(12), "  est")]
fn char_boundaries_test(#[case] start: Option<usize>, #[case] end: Option<usize>, #[case] expected: &str) {
    let mut styled = get_styled_text("  ąęśćńół: test");
    styled[0].sl_drain(start, end);
    assert_eq!(expected, styled.to_string());
}

#[rstest]
#[case(None, Some(5), "rsion: v1 #with comment")]
#[case(None, Some(5), "rsion: v1 #with comment")]
#[case(None, Some(9), "n: v1 #with comment")]
#[case(None, Some(10), ": v1 #with comment")]
#[case(None, Some(11), " v1 #with comment")]
#[case(None, Some(13), "1 #with comment")]
#[case(None, Some(18), "th comment")]
#[case(Some(3), Some(6), "apision: v1 #with comment")]
#[case(Some(3), Some(12), "apiv1 #with comment")]
#[case(Some(3), Some(18), "apith comment")]
#[case(Some(10), Some(11), "apiVersion v1 #with comment")]
#[case(Some(10), Some(16), "apiVersionwith comment")]
#[case(Some(10), Some(29), "apiVersion")]
#[case(Some(10), Some(30), "apiVersion")]
#[case(Some(10), None, "apiVersion")]
#[case(Some(11), None, "apiVersion:")]
#[case(Some(12), None, "apiVersion: ")]
#[case(Some(14), None, "apiVersion: v1")]
#[case(Some(17), None, "apiVersion: v1 #w")]
#[case(Some(30), None, "apiVersion: v1 #with comment")]
#[case(Some(100), Some(150), "apiVersion: v1 #with comment")]
fn sl_drain_test(#[case] start: Option<usize>, #[case] end: Option<usize>, #[case] expected: &str) {
    let mut styled = get_styled_text("apiVersion: v1 #with comment");
    styled[0].sl_drain(start, end);
    assert_eq!(expected, styled.to_string());
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

#[test]
fn insert_text_test() {
    let styles = StyleFallback {
        excluded: Style::default(),
        fallback: Style::default(),
    };
    let yaml = r"apiVersion: v1
kind: Pod
metadata:
  creationTimestamp: 2025-08-27T19:31:08Z
  generateName: coredns-6799fbcd5-";
    let mut actual = get_styled_text(yaml);

    let to_insert = r"_lines
to insert
into the yaml_";
    let text = to_insert.split('\n').map(String::from).collect::<Vec<_>>();
    actual.insert_text(ContentPosition::new(5, 3), &text, &styles);

    let expected = r"apiVersion: v1
kind: Pod
metadata:
  cre_lines
to insert
into the yaml_ationTimestamp: 2025-08-27T19:31:08Z
  generateName: coredns-6799fbcd5-";

    assert_eq!(expected, actual.to_string());
}
