use super::*;
use crate::{LogParser, parse_line};

#[test]
fn parses_prometheus_operator_startup() {
    let line = r#"ts=2026-10-05T05:29:03.691095335Z level=info caller=/workspace/cmd/operator/main.go:290 msg="Starting Prometheus Operator" version="(version=0.91.0, branch=, revision=e138807)" build_context="(go=go1.25.9, platform=linux/amd64, user=, date=20260505-08:53:40, tags=unknown)" feature_gates="PrometheusAgentDaemonSet=false,PrometheusShardRetentionPolicy=false""#;
    let parsed = parse(line).unwrap();

    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("Starting Prometheus Operator", parsed.message);
    assert_eq!(
        Some(
            r#"caller=/workspace/cmd/operator/main.go:290, version="(version=0.91.0, branch=, revision=e138807)", build_context="(go=go1.25.9, platform=linux/amd64, user=, date=20260505-08:53:40, tags=unknown)", feature_gates="PrometheusAgentDaemonSet=false,PrometheusShardRetentionPolicy=false""#
        ),
        parsed.context.as_deref(),
    );
}

#[test]
fn parses_empty_and_boolean_fields() {
    let line = r#"ts=2026-10-05T05:29:03.691237563Z level=info caller=/workspace/cmd/operator/main.go:291 msg="Operator's configuration" watch_referenced_objects_in_all_namespaces=false controller_id="" enable_config_reloader_probes=false"#;
    let parsed = parse(line).unwrap();

    assert_eq!("Operator's configuration", parsed.message);
    assert_eq!(
        Some(
            r#"caller=/workspace/cmd/operator/main.go:291, watch_referenced_objects_in_all_namespaces=false, controller_id="", enable_config_reloader_probes=false"#
        ),
        parsed.context.as_deref(),
    );
}

#[test]
fn parses_empty_quoted_field_without_consuming_the_next_field() {
    let (rest, (key, value)) = parse_field(r#"controller_id="" enable_config_reloader_probes=false"#).unwrap();

    assert_eq!("controller_id", key);
    assert_eq!("", value);
    assert_eq!(" enable_config_reloader_probes=false", rest);
}

#[test]
fn preserves_escaped_context_and_message_spaces() {
    let parsed =
        parse(r#"level=info msg="Namespaces filtering configuration " config="{allow_list=\"\",deny_list=\"\"}""#).unwrap();

    assert_eq!("Namespaces filtering configuration ", parsed.message);
    assert_eq!(
        Some(r#"config="{allow_list=\"\",deny_list=\"\"}""#),
        parsed.context.as_deref()
    );
}

#[test]
fn parses_warning_and_error_context() {
    let parsed = parse(r#"ts=2026-10-05T05:29:03.799363113Z level=warn caller=/workspace/pkg/server/server.go:193 msg="server TLS client verification disabled" client_ca_file=/etc/tls/private/tls-ca.crt err="stat /etc/tls/private/tls-ca.crt: no such file or directory""#).unwrap();

    assert_eq!(LogLevel::Warn, parsed.level);
    assert_eq!("server TLS client verification disabled", parsed.message);
    assert_eq!(
        Some(
            r#"caller=/workspace/pkg/server/server.go:193, client_ca_file=/etc/tls/private/tls-ca.crt, err="stat /etc/tls/private/tls-ca.crt: no such file or directory""#
        ),
        parsed.context.as_deref()
    );
}

#[test]
fn decodes_message_escapes() {
    let parsed = parse(r#"level=error msg="say \"hello\" at C:\\logs\nnext\tcolumn\r""#).unwrap();

    assert_eq!(LogLevel::Error, parsed.level);
    assert_eq!("say \"hello\" at C:\\logs\nnext\tcolumn\r", parsed.message);
    assert!(parsed.context.is_none());
}

#[test]
fn parses_unicode_without_corrupting_field_boundaries() {
    let line = format!(
        r#"level=info msg="{} \"quoted\"" name="{}""#,
        "caf\u{00e9}", "\u{6771}\u{4eac}"
    );
    let parsed = parse(&line).unwrap();
    assert_eq!("caf\u{00e9} \"quoted\"", parsed.message);
    assert_eq!(Some("name=\"\u{6771}\u{4eac}\""), parsed.context.as_deref());
}

#[test]
fn accepts_unquoted_and_empty_messages_with_reordered_fields() {
    let parsed = parse("name=pod msg=ready level=INFO").unwrap();
    assert_eq!(LogLevel::Info, parsed.level);
    assert_eq!("ready", parsed.message);
    assert_eq!(Some("name=pod"), parsed.context.as_deref());

    for line in [r#"level=debug msg="""#, "level=debug msg="] {
        assert_eq!("", parse(line).unwrap().message);
    }
}

#[test]
fn accepts_whitespace_and_omits_timestamps() {
    let parsed = parse(" \tlevel=info\tmsg=ready time=now timestamp=now ts=now \t").unwrap();
    assert_eq!("ready", parsed.message);
    assert!(parsed.context.is_none());
}

#[test]
fn rejects_unstructured_or_malformed_lines() {
    for line in [
        "",
        "plain text",
        "name=pod",
        "level=info",
        "msg=ready",
        "level=unknown msg=ready",
        r#"level=info msg="unfinished"#,
        r#"level=info msg="bad\q""#,
        r#"level=info msg="ready"suffix"#,
        r#"level=info msg="ready"name=pod"#,
        "level=info msg=ready trailing",
        "level=info msg=ready =value",
        "level=info msg=un\"quoted",
    ] {
        assert!(parse(line).is_none(), "{line}");
    }
}

#[test]
fn detects_logfmt_and_switches_between_text_formats() {
    let logfmt = r#"level=info msg="ready""#;
    let klog = r#"I1003 20:16:48.502874 1 main.go:42] "ready""#;

    for previous in [None, Some(LogParser::Klog), Some(LogParser::Logfmt), Some(LogParser::Zap)] {
        assert_eq!(LogParser::Logfmt, parse_line(logfmt, previous).unwrap().1);
        assert_eq!(LogParser::Klog, parse_line(klog, previous).unwrap().1);
        assert!(parse_line("plain text", previous).is_none());
    }
}

#[test]
fn detects_json_after_either_text_format() {
    for previous in [Some(LogParser::Klog), Some(LogParser::Logfmt)] {
        let (parsed, parser) = parse_line(r#"{"level":"info","msg":"ready","ts":1}"#, previous).unwrap();
        assert_eq!(LogParser::Zap, parser);
        assert_eq!("ready", parsed.message);
    }
}
