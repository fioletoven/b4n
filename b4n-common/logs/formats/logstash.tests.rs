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
    assert_eq!(Some("com.example.HttpServer, thread=main"), parsed.context.as_deref());
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
        Some("com.example.Requests, request=req-7, span=def456, thread=http-nio-8080-exec-4, trace=abc123"),
        parsed.context.as_deref(),
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
    assert_eq!(Some("trace=trace-1"), parsed.context.as_deref());
}

#[test]
fn collects_logstash_tags_caller_and_stacktrace() {
    let parsed = parse(
        json!({
            "message": "failed",
            "level": "ERROR",
            "HOSTNAME": "worker-0",
            "tags": ["audit", "http"],
            "caller_class_name": "com.example.Worker",
            "caller_method_name": "run",
            "caller_file_name": "Worker.java",
            "caller_line_number": 27,
            "stack_hash": "abc123",
            "stack_trace": "Exception: failed\n at Worker.run()"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        Some(concat!(
            "class=com.example.Worker, file=Worker.java, line=27, method=run, host=worker-0, stack_hash=abc123, ",
            "stack=Exception: failed\n at Worker.run(), tags=[\"audit\",\"http\"]"
        )),
        parsed.context.as_deref(),
    );
}

#[test]
fn supports_snake_case_correlation_ids_without_duplicates() {
    let parsed = parse(
        json!({
            "message": "failed",
            "level": "ERROR",
            "traceId": "preferred-trace",
            "trace_id": "alternate-trace",
            "spanId": "",
            "span_id": "span-1",
            "requestId": null,
            "request_id": "req-1",
            "correlation_id": "correlation-1",
            "tags": [],
            "stack_trace": ["Exception: failed", "at Worker.run()"]
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        Some(concat!(
            "correlation=correlation-1, request=req-1, span=span-1, ",
            "stack=[\"Exception: failed\",\"at Worker.run()\"], trace=preferred-trace"
        )),
        parsed.context.as_deref(),
    );
}
