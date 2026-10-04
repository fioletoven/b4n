use super::*;

#[test]
fn parses_info_line_with_single_field() {
    let line =
        r#"I1003 20:16:48.502874       1 controller.go:838] "Starting provisioner controller" component="rancher.io/local-path""#;
    let parsed = parse(line).unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("Starting provisioner controller", parsed.message);
    assert_eq!(
        Some(r#"controller.go:838, tid=1, component="rancher.io/local-path""#),
        parsed.context.as_deref(),
    );
}

#[test]
fn parses_warning_level() {
    let line = r#"W1003 20:16:48.502874 1 reflector.go:150] "watch closed" reason="unexpected EOF""#;
    let parsed = parse(line).unwrap();

    assert_eq!(LogLevel::Warn, parsed.level);
    assert_eq!("watch closed", parsed.message);
}

#[test]
fn parses_error_level() {
    let line = r#"E1003 20:16:48.502874 1 controller.go:200] "sync failed" error="not found""#;
    let parsed = parse(line).unwrap();

    assert_eq!(LogLevel::Error, parsed.level);
}

#[test]
fn parses_fatal_level() {
    let line = r#"F1003 20:16:48.502874 1 main.go:42] "unable to start" error="bind: address already in use""#;
    let parsed = parse(line).unwrap();

    assert_eq!(LogLevel::Fatal, parsed.level);
}

#[test]
fn parses_message_with_multiple_fields() {
    let line = r#"I1003 20:16:48.502874   5 controller.go:838] "reconciling" name="foo" namespace="default" attempt=3"#;
    let parsed = parse(line).unwrap();

    assert_eq!("reconciling", parsed.message);
    assert_eq!(
        Some(r#"controller.go:838, tid=5, name="foo", namespace="default", attempt=3"#),
        parsed.context.as_deref(),
    );
}

#[test]
fn parses_message_without_any_fields() {
    let line = r#"I1003 20:16:48.502874 1 controller.go:838] "Started provisioner controller""#;
    let parsed = parse(line).unwrap();

    assert_eq!("Started provisioner controller", parsed.message);
    assert_eq!(Some("controller.go:838, tid=1"), parsed.context.as_deref());
}

#[test]
fn falls_back_to_raw_remainder_when_message_is_not_quoted() {
    let line = "I1003 20:16:48.502874       999 controller.go:838] plain unquoted message";
    let parsed = parse(line).unwrap();

    assert_eq!("plain unquoted message", parsed.message);
    assert_eq!(Some("controller.go:838, tid=999"), parsed.context.as_deref());
}

#[test]
fn field_value_with_quoted_spaces_is_kept_as_single_field() {
    let line = r#"I1003 20:16:48.502874 1 controller.go:838] "sync failed" error="connection refused: timed out""#;
    let parsed = parse(line).unwrap();

    assert_eq!(
        Some(r#"controller.go:838, tid=1, error="connection refused: timed out""#),
        parsed.context.as_deref(),
    );
}

#[test]
fn field_value_with_bracketed_spaces_is_kept_as_single_field() {
    let line = r#"I1003 20:16:48.502874 1 controller.go:838] "applying patch" items=[a b c] name=foo"#;
    let parsed = parse(line).unwrap();

    assert_eq!(
        Some("controller.go:838, tid=1, items=[a b c], name=foo"),
        parsed.context.as_deref(),
    );
}

#[test]
fn field_value_with_nested_go_map_is_kept_as_single_field() {
    let line = r#"I1003 20:16:48.502874 1 controller.go:838] "applying patch" meta=map[key:val other:map[k:v]]"#;
    let parsed = parse(line).unwrap();

    assert_eq!(
        Some("controller.go:838, tid=1, meta=map[key:val other:map[k:v]]"),
        parsed.context.as_deref(),
    );
}

#[test]
fn rejects_line_with_invalid_level_letter() {
    let line = r#"X1003 20:16:48.502874 1 controller.go:838] "Starting""#;

    assert!(parse(line).is_none());
}

#[test]
fn rejects_line_missing_closing_bracket() {
    let line = r#"I1003 20:16:48.502874 1 controller.go:838 "Starting""#;

    assert!(parse(line).is_none());
}

#[test]
fn rejects_line_with_malformed_timestamp() {
    let line = r#"I1003 2a:16:48.502874 1 controller.go:838] "Starting""#;

    assert!(parse(line).is_none());
}
