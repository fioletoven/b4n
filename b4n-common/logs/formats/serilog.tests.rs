use super::*;
use serde_json::json;

#[test]
fn uses_rendered_message_as_is() {
    let parsed = parse(
        json!({
            "@t": "2026-09-29T10:00:00Z",
            "@l": "Warning",
            "@m": "plain rendered message",
            "Name": "ignored",
            "SourceContext": "Api.Worker",
            "ThreadId": 9
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Warn, parsed.level);
    assert_eq!("plain rendered message", parsed.message);
    assert_eq!(Some("Api.Worker, tid=9"), parsed.context.as_deref());
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
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("Hello world, count=3, data={\"ok\":true}", parsed.message);
    assert_eq!(None, parsed.context);
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
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("{Value} present {Missing:000}", parsed.message);
    assert_eq!(None, parsed.context);
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
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("Job 42 finished", parsed.message);
    assert_eq!(None, parsed.context);
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
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("User {\"name\":\"alice\"} payload raw", parsed.message);
    assert_eq!(None, parsed.context);
}

#[test]
fn collects_common_serilog_context_fields() {
    let parsed = parse(
        json!({
            "@mt": "Started",
            "SourceContext": "Test.Namespace",
            "MachineName": "some-pod-0",
            "EnvironmentName": "Production",
            "ThreadId": 1,
            "TraceId": "abc123",
            "SpanId": "def456"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        Some("Test.Namespace, env=Production, machine=some-pod-0, span=def456, tid=1, trace=abc123"),
        parsed.context.as_deref(),
    );
}

#[test]
fn skips_empty_context_values() {
    let parsed = parse(
        json!({
            "@mt": "Started",
            "SourceContext": "",
            "MachineName": null,
            "ThreadId": 4
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(Some("tid=4"), parsed.context.as_deref());
}

#[test]
fn collects_compact_correlation_and_exception_fields() {
    let parsed = parse(
        json!({
            "@m": "Failed",
            "@tr": "compact-trace",
            "@sp": "compact-span",
            "TraceId": "enriched-trace",
            "SpanId": "enriched-span",
            "Application": "api",
            "ProcessId": 42,
            "ThreadName": "worker",
            "CorrelationId": "correlation-1",
            "RequestPath": "/orders",
            "@i": "event-1",
            "@x": "System.Exception: failed\n at Worker.Run()"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        Some(concat!(
            "app=api, correlation=correlation-1, error=System.Exception: failed\n at Worker.Run(), event=event-1, ",
            "pid=42, path=/orders, span=compact-span, thread=worker, trace=compact-trace"
        )),
        parsed.context.as_deref(),
    );
}

#[test]
fn falls_back_to_enriched_ids_when_compact_ids_are_blank() {
    let parsed = parse(
        json!({
            "@m": "Started",
            "@tr": "",
            "@sp": null,
            "TraceId": "trace-1",
            "SpanId": "span-1",
            "@x": null,
            "ProcessId": null
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(Some("span=span-1, trace=trace-1"), parsed.context.as_deref());
}
