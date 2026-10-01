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
        Some("gateway.access, thread=http-8080-2, service=gateway, span=span-1, trace=trace-1"),
        parsed.context.as_deref(),
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
        Some("jobs.runner, thread=pool-3-thread-1, service=worker, trace=trace-7, transaction=txn-4"),
        parsed.context.as_deref(),
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
    assert_eq!(Some("service=metrics"), parsed.context.as_deref());
}

#[test]
fn collects_extended_ecs_context_in_nested_and_flattened_logs() {
    let lines = [
        json!({
            "message": "failed",
            "log": { "level": "error", "origin": { "file": { "name": "worker.rs", "line": 27 }, "function": "run" } },
            "service": { "environment": "production", "version": "1.2", "node": { "name": "worker-0" } },
            "host": { "name": "node-1" },
            "process": { "pid": 42, "thread": { "id": 7 } },
            "event": { "dataset": "worker.log" },
            "error": { "type": "Timeout", "message": "timed out", "stack_trace": "run\nmain" }
        }),
        json!({
            "message": "failed",
            "log.level": "error",
            "service.environment": "production",
            "service.version": "1.2",
            "service.node.name": "worker-0",
            "host.name": "node-1",
            "process.pid": 42,
            "process.thread.id": 7,
            "event.dataset": "worker.log",
            "log.origin.file.name": "worker.rs",
            "log.origin.file.line": 27,
            "log.origin.function": "run",
            "error.type": "Timeout",
            "error.message": "timed out",
            "error.stack_trace": "run\nmain"
        }),
    ];

    for line in lines {
        let map = line.as_object().unwrap();
        assert!(detect(map));
        let parsed = parse(map).unwrap();
        assert_eq!(LogLevel::Error, parsed.level);
        assert_eq!(
            Some(concat!(
                "error=timed out, stack=run\nmain, error_type=Timeout, dataset=worker.log, host=node-1, line=27, ",
                "file=worker.rs, function=run, pid=42, tid=7, env=production, node=worker-0, version=1.2"
            )),
            parsed.context.as_deref(),
        );
    }
}

#[test]
fn prefers_flattened_ecs_values_and_skips_blank_metadata() {
    let parsed = parse(
        json!({
            "message": "started",
            "log.level": "info",
            "process.pid": 0,
            "process": { "pid": 42 },
            "service.environment": "",
            "host.name": null,
            "error": { "message": "", "stack_trace": null }
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(Some("pid=0"), parsed.context.as_deref());
}
