use super::*;
use serde_json::json;

#[test]
fn parses_log4j2_json_layout() {
    let parsed = parse(
        json!({
            "instant": { "epochSecond": 1_727_604_000, "nanoOfSecond": 0 },
            "thread": "main",
            "level": "ERROR",
            "loggerName": "com.example.Startup",
            "message": "Application failed to start"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Error, parsed.level);
    assert_eq!("Application failed to start", parsed.message);
    assert_eq!(Some("com.example.Startup, thread=main"), parsed.context.as_deref());
}

#[test]
fn collects_common_log4j2_context_fields() {
    let parsed = parse(
        json!({
            "thread": "http-nio-8080-exec-2",
            "threadId": 18,
            "level": "WARN",
            "loggerName": "com.example.RequestLogger",
            "loggerFqcn": "org.apache.logging.log4j.spi.AbstractLogger",
            "endOfBatch": false,
            "message": "Slow request detected"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Warn, parsed.level);
    assert_eq!(
        Some(concat!(
            "com.example.RequestLogger, batch=false, fqcn=org.apache.logging.log4j.spi.AbstractLogger, ",
            "thread=http-nio-8080-exec-2, tid=18"
        )),
        parsed.context.as_deref(),
    );
}

#[test]
fn skips_empty_context_values() {
    let parsed = parse(
        json!({
            "thread": "",
            "level": "INFO",
            "loggerName": null,
            "message": "started"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!(None, parsed.context);
}

#[test]
fn collects_log4j2_diagnostic_context_and_exception() {
    let parsed = parse(
        json!({
            "message": "failed",
            "level": "ERROR",
            "thread": "main",
            "threadPriority": 5,
            "marker": { "name": "AUDIT" },
            "contextMap": { "traceId": "trace-1" },
            "contextStack": ["request-1"],
            "source": { "file": "Worker.java", "line": 27 },
            "thrown": { "name": "java.lang.IllegalStateException", "message": "failed" }
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        Some(concat!(
            "mdc={\"traceId\":\"trace-1\"}, ndc=[\"request-1\"], marker={\"name\":\"AUDIT\"}, ",
            "source={\"file\":\"Worker.java\",\"line\":27}, thread=main, priority=5, ",
            "error={\"message\":\"failed\",\"name\":\"java.lang.IllegalStateException\"}"
        )),
        parsed.context.as_deref()
    );
}

#[test]
fn supports_log4j2_context_map_as_list_and_string_stacktrace() {
    let parsed = parse(
        json!({
            "message": "failed",
            "level": "ERROR",
            "contextMap": [{ "key": "requestId", "value": "req-1" }],
            "contextStack": [],
            "marker": {},
            "source": null,
            "thrown": "Exception: failed\n at Worker.run()"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        Some("mdc=[{\"key\":\"requestId\",\"value\":\"req-1\"}], error=Exception: failed\n at Worker.run()"),
        parsed.context.as_deref(),
    );
}
