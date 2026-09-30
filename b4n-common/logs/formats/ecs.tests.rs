use super::*;
use serde_json::json;

#[test]
fn parses_nested_ecs_logs() {
    let parsed = parse(
        json!({
            "@timestamp": "2026-09-29T10:00:00.000Z",
            "message": "request finished",
            "log": { "level": "info", "logger": "gateway.access" },
            "service": { "name": "gateway" },
            "process": { "thread": { "name": "http-8080-2" } },
            "trace": { "id": "trace-1" },
            "span": { "id": "span-1" }
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("request finished", parsed.message);
    assert_eq!(
        Some("gateway.access, service=gateway, thread=http-8080-2, trace=trace-1, span=span-1".to_owned()),
        parsed.context,
    );
}

#[test]
fn parses_flattened_ecs_logs() {
    let parsed = parse(
        json!({
            "message": "job failed",
            "log.level": "error",
            "log.logger": "jobs.runner",
            "service.name": "worker",
            "process.thread.name": "pool-3-thread-1",
            "trace.id": "trace-7",
            "transaction.id": "txn-4"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Error, parsed.level);
    assert_eq!(
        Some("jobs.runner, service=worker, thread=pool-3-thread-1, trace=trace-7, transaction=txn-4".to_owned()),
        parsed.context,
    );
}

#[test]
fn skips_empty_ecs_context_values() {
    let parsed = parse(
        json!({
            "message": "heartbeat",
            "log": { "level": "debug", "logger": "" },
            "service": { "name": "metrics" },
            "process.thread.name": ""
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Debug, parsed.level);
    assert_eq!(Some("service=metrics".to_owned()), parsed.context);
}
