use super::*;
use b4n_config::themes::LogsSyntaxColors;
use k8s_openapi::jiff::Timestamp;

use crate::ui::views::logs::line::LogLine;

fn make_line(datetime: &str, message: &str) -> LogLine {
    LogLine::new(datetime.parse::<Timestamp>().unwrap(), None, message.to_owned())
}

fn make_line_with_container(datetime: &str, container: &str, message: &str) -> LogLine {
    LogLine::new(datetime.parse::<Timestamp>().unwrap(), Some(container), message.to_owned())
}

fn make_error_line(datetime: &str, message: &str) -> LogLine {
    LogLine::error(datetime.parse::<Timestamp>().unwrap(), None, message.to_owned())
}

fn messages(content: &LogsContent) -> Vec<&str> {
    content.lines.iter().map(|l| l.lowercase.as_str()).collect()
}

#[test]
fn add_single_line() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:00Z", "hello"));

    assert_eq!(content.len(), 1);
    assert_eq!(messages(&content), vec!["hello"]);
}

#[test]
fn add_lines_in_order() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "first"));
    content.add_log_line(make_line("2024-01-01T00:00:02Z", "second"));
    content.add_log_line(make_line("2024-01-01T00:00:03Z", "third"));

    assert_eq!(content.len(), 3);
    assert_eq!(messages(&content), vec!["first", "second", "third"]);
}

#[test]
fn add_lines_reverse_order() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:03Z", "third"));
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "first"));
    content.add_log_line(make_line("2024-01-01T00:00:02Z", "second"));

    assert_eq!(content.len(), 3);
    assert_eq!(messages(&content), vec!["first", "second", "third"]);
}

#[test]
fn add_lines_interleaved() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "a"));
    content.add_log_line(make_line("2024-01-01T00:00:05Z", "e"));
    content.add_log_line(make_line("2024-01-01T00:00:03Z", "c"));
    content.add_log_line(make_line("2024-01-01T00:00:02Z", "b"));
    content.add_log_line(make_line("2024-01-01T00:00:04Z", "d"));

    assert_eq!(content.len(), 5);
    assert_eq!(messages(&content), vec!["a", "b", "c", "d", "e"]);
}

#[test]
fn lines_sorted_by_timestamp_then_container() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line_with_container("2024-01-01T00:00:01Z", "beta", "b"));
    content.add_log_line(make_line_with_container("2024-01-01T00:00:01Z", "alpha", "a"));

    assert_eq!(content.len(), 2);
    assert_eq!(messages(&content), vec!["a", "b"]);
}

#[test]
fn duplicate_line_appended_is_ignored() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "hello"));
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "hello"));

    assert_eq!(content.len(), 1);
}

#[test]
fn duplicate_line_inserted_is_ignored() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "first"));
    content.add_log_line(make_line("2024-01-01T00:00:03Z", "third"));
    // insert duplicate of "first" via merge_sorted path
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "first"));

    assert_eq!(content.len(), 2);
    assert_eq!(messages(&content), vec!["first", "third"]);
}

#[test]
fn same_timestamp_different_message_not_deduplicated() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "aaa"));
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "bbb"));

    assert_eq!(content.len(), 2);
}

#[test]
fn same_timestamp_different_kind_not_deduplicated() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "msg"));
    content.add_log_line(make_error_line("2024-01-01T00:00:01Z", "msg"));

    assert_eq!(content.len(), 2);
}

#[test]
fn same_timestamp_different_container_not_deduplicated() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line_with_container("2024-01-01T00:00:01Z", "app", "msg"));
    content.add_log_line(make_line_with_container("2024-01-01T00:00:01Z", "sidecar", "msg"));

    assert_eq!(content.len(), 2);
}

#[test]
fn get_first_line_skips_non_log_lines() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_error_line("2024-01-01T00:00:01Z", "error"));
    content.add_log_line(make_line("2024-01-01T00:00:02Z", "real log"));

    let first = content.get_first_line().unwrap();
    assert_eq!(first.lowercase, "real log");
}

#[test]
fn many_lines_same_timestamp() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    for i in 0..100 {
        content.add_log_line(make_line("2024-01-01T00:00:01Z", &format!("line {i}")));
    }

    assert_eq!(content.len(), 100);
}

#[test]
fn many_duplicate_lines_same_timestamp() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    for _ in 0..100 {
        content.add_log_line(make_line("2024-01-01T00:00:01Z", "same"));
    }

    assert_eq!(content.len(), 1);
}

#[test]
fn add_line_to_middle() {
    let mut content = LogsContent::new(LogsSyntaxColors::default());
    content.add_log_line(make_line("2024-01-01T00:00:01Z", "first"));
    content.add_log_line(make_line("2024-01-01T00:00:05Z", "last"));
    content.add_log_line(make_line("2024-01-01T00:00:03Z", "middle"));

    assert_eq!(messages(&content), vec!["first", "middle", "last"]);
}
