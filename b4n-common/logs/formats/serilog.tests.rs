use super::*;
use serde_json::json;

#[test]
fn uses_rendered_message_as_is() {
    let parsed = parse(
        json!({
            "@t": "2026-09-29T10:00:00Z",
            "@l": "Warning",
            "@m": "plain rendered message",
            "Name": "ignored"
        })
        .as_object()
        .unwrap(),
    );

    assert_eq!(Some("[ WARN]  plain rendered message".to_string()), parsed);
}

#[test]
fn renders_message_template_from_properties() {
    let parsed = parse(
        json!({
            "@t": "2026-09-29T10:00:00Z",
            "@l": "Information",
            "@mt": "Hello {Name}, count={Count}, data={Data}",
            "Name": "world",
            "Count": 3,
            "Data": { "ok": true }
        })
        .as_object()
        .unwrap(),
    );

    assert_eq!(Some("[ INFO]  Hello world, count=3, data={\"ok\":true}".to_string()), parsed);
}

#[test]
fn preserves_missing_properties_and_escaped_braces() {
    let parsed = parse(
        json!({
            "@t": "2026-09-29T10:00:00Z",
            "@mt": "{{Value}} {Known} {Missing:000}",
            "Known": "present"
        })
        .as_object()
        .unwrap(),
    );

    assert_eq!(Some("[ INFO]  {Value} present {Missing:000}".to_string()), parsed);
}

#[test]
fn detects_and_renders_without_timestamp_field() {
    let parsed = parse(
        json!({
            "@mt": "Job {Id} finished",
            "Id": 42
        })
        .as_object()
        .unwrap(),
    );

    assert_eq!(Some("[ INFO]  Job 42 finished".to_string()), parsed);
}

#[test]
fn supports_serilog_operators_in_placeholders() {
    let parsed = parse(
        json!({
            "@mt": "User {@User} payload {$Payload}",
            "User": { "name": "alice" },
            "Payload": "raw"
        })
        .as_object()
        .unwrap(),
    );

    assert_eq!(Some("[ INFO]  User {\"name\":\"alice\"} payload raw".to_string()), parsed);
}
