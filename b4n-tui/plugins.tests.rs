use super::*;

fn make_context() -> PluginContext {
    PluginContext {
        context: "my-cluster".to_string(),
        kind: Kind::new("deployments", "apps", "v1"),
        namespace: Namespace::from("default"),
        highlighted: None,
        selected: vec![],
        columns: vec!["NAME".to_string(), "IMAGE".to_string(), "STATUS".to_string()],
        values: vec![
            vec!["my-pod".to_string(), "nginx:latest".to_string(), "Running".to_string()],
            vec!["other-pod".to_string(), "alpine:3".to_string(), "Pending".to_string()],
        ],
    }
}

#[test]
fn test_col_substitution() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[IMAGE]", 0), "nginx:latest");
    assert_eq!(ctx.resolve_arg("$COL[IMAGE]", 1), "alpine:3");
}

#[test]
fn test_col_case_insensitive() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[image]", 0), "nginx:latest");
    assert_eq!(ctx.resolve_arg("$COL[Image]", 0), "nginx:latest");
}

#[test]
fn test_simple_placeholders() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$CONTEXT", 0), "my-cluster");
    assert_eq!(ctx.resolve_arg("$NAMESPACE", 0), "default");
    assert_eq!(ctx.resolve_arg("$PLURAL", 0), "deployments");
    assert_eq!(ctx.resolve_arg("$GROUP", 0), "apps");
    assert_eq!(ctx.resolve_arg("$VERSION", 0), "v1");
}

#[test]
fn test_mixed_arg() {
    let ctx = make_context();
    assert_eq!(
        ctx.resolve_arg("$NAMESPACE/$COL[NAME]:$COL[STATUS]", 0),
        "default/my-pod:Running"
    );
}

#[test]
fn test_no_placeholders() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("just a plain string", 0), "just a plain string");
    assert_eq!(ctx.resolve_arg("", 0), "");
}

#[test]
fn test_unknown_placeholder_kept_literal() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$UNKNOWN", 0), "$UNKNOWN");
}

#[test]
fn test_col_unknown_column_returns_empty() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[NONEXISTENT]", 0), "");
}

#[test]
fn test_col_missing_closing_bracket_kept_literal() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[IMAGE", 0), "$COL[IMAGE");
}

#[test]
fn test_col_out_of_bounds_row() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[IMAGE]", 99), "");
}

#[test]
fn test_dollar_sign_literal() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("costs $5 dollars", 0), "costs $5 dollars");
}

#[test]
fn test_multiple_same_placeholder() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[NAME]-$COL[NAME]", 0), "my-pod-my-pod");
}
