use super::*;
use serde_json::json;

#[test]
fn parses_logstash_logback_layout() {
    let parsed = parse(
        json!({
            "@timestamp": "2026-09-29T10:00:00.000Z",
            "@version": "1",
            "message": "HTTP server started",
            "logger_name": "com.example.HttpServer",
            "thread_name": "main",
            "level": "INFO"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("HTTP server started", parsed.message);
    assert_eq!(Some("com.example.HttpServer, thread=main".to_owned()), parsed.context);
}

#[test]
fn parses_context_with_trace_identifiers() {
    let parsed = parse(
        json!({
            "@timestamp": "2026-09-29T10:00:00.000Z",
            "message": "Handled request",
            "logger_name": "com.example.Requests",
            "thread_name": "http-nio-8080-exec-4",
            "level": "WARN",
            "traceId": "abc123",
            "spanId": "def456",
            "requestId": "req-7"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Warn, parsed.level);
    assert_eq!(
        Some("com.example.Requests, thread=http-nio-8080-exec-4, trace=abc123, span=def456, request=req-7".to_owned()),
        parsed.context,
    );
}

#[test]
fn skips_empty_context_values() {
    let parsed = parse(
        json!({
            "@timestamp": "2026-09-29T10:00:00.000Z",
            "message": "Handled request",
            "logger_name": "",
            "thread_name": null,
            "level": "DEBUG",
            "traceId": "trace-1"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Debug, parsed.level);
    assert_eq!(Some("trace=trace-1".to_owned()), parsed.context);
}
