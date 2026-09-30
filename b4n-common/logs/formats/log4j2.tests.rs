use super::*;
use serde_json::json;

#[test]
fn parses_log4j2_json_layout() {
    let parsed = parse(
        json!({
            "instant": { "epochSecond": 1727604000, "nanoOfSecond": 0 },
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
    assert_eq!(Some("com.example.Startup, thread=main".to_owned()), parsed.context);
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
        Some("com.example.RequestLogger, thread=http-nio-8080-exec-2, thread_id=18, batch=false, fqcn=org.apache.logging.log4j.spi.AbstractLogger".to_owned()),
        parsed.context,
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
