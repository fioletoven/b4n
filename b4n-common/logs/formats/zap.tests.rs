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
    assert_eq!(Some("http.server, caller=server/main.go:42".to_owned()), parsed.context);
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
        Some("worker, caller=worker/run.go:10, stack=main.main\nworker.run".to_owned()),
        parsed.context,
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
