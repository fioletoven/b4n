use super::*;
use serde_json::json;

#[test]
fn parses_zap_json_logs() {
    let parsed = parse(
        json!({
            "level": "info",
            "ts": 1727604000.123,
            "logger": "http.server",
            "caller": "server/main.go:42",
            "msg": "listening"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("listening", parsed.message);
    assert_eq!(Some("http.server, caller=server/main.go:42"), parsed.context.as_deref());
}

#[test]
fn includes_stacktrace_in_context() {
    let parsed = parse(
        json!({
            "level": "error",
            "ts": 1727604000.123,
            "logger": "worker",
            "caller": "worker/run.go:10",
            "stacktrace": "main.main\nworker.run",
            "msg": "crashed"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Error, parsed.level);
    assert_eq!(
        Some("worker, caller=worker/run.go:10, stack=main.main\nworker.run"),
        parsed.context.as_deref(),
    );
}

#[test]
fn skips_empty_context_values() {
    let parsed = parse(
        json!({
            "level": "debug",
            "ts": 1727604000.123,
            "logger": "",
            "caller": null,
            "msg": "tick"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Debug, parsed.level);
    assert_eq!(None, parsed.context);
}

#[test]
fn collects_controller_runtime_context_fields() {
    let parsed = parse(
        json!({
            "level": "error",
            "ts": "2026-10-01T07:33:46Z",
            "msg": "Failed to Get the role Info of the",
            "controller": "redisreplication",
            "controllerGroup": "redis.redis.opstreelabs.in",
            "controllerKind": "RedisReplication",
            "namespace": "test",
            "name": "redis",
            "reconcileID": "7acbdc55-2aa7-4634-afe9-6e31a4aba1d1",
            "redis pod": "redis-0",
            "error": "dial tcp :6379: connect: connection refused"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Error, parsed.level);
    assert_eq!(
        Some(concat!(
            "controller=redisreplication, group=redis.redis.opstreelabs.in, kind=RedisReplication, ",
            "error=dial tcp :6379: connect: connection refused, name=redis, ns=test, reconcile=7acbdc55-2aa7-4634-afe9-6e31a4aba1d1"
        )),
        parsed.context.as_deref(),
    );
}

#[test]
fn detects_zap_variant_with_time_field() {
    let parsed = parse(
        json!({
            "time": "2026-10-01T09:44:31.885093+00:00",
            "level": "INFO",
            "msg": "Found a folder override annotation"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("Found a folder override annotation", parsed.message,);
    assert_eq!(None, parsed.context);
}

#[test]
fn collects_zap_pod_and_correlation_fields() {
    let parsed = parse(
        json!({
            "msg": "request failed",
            "level": "error",
            "ts": 1727604000.123,
            "pod": "api-0",
            "trace_id": "trace-1",
            "span_id": "span-1",
            "request_id": "req-1",
            "error": "timed out"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        Some("error=timed out, pod=api-0, request=req-1, span=span-1, trace=trace-1"),
        parsed.context.as_deref(),
    );
}
