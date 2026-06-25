use super::*;

fn make_context() -> PluginContext {
    PluginContext {
        context: "my-cluster".to_string(),
        kind: Kind::new("deployments", "apps", "v1"),
        namespace: Namespace::from("default"),
        resources: vec![
            ResourceRef::container("my-pod".to_string(), Namespace::from("default"), "main".to_string())
                .with_uid("uid-1234".to_string()),
            ResourceRef::named(
                Kind::new("pods", "core", "v1"),
                Namespace::from("kube-system"),
                "other-pod".to_string(),
            )
            .with_uid("uid-5678".to_string()),
        ],
        columns: vec!["NAME".to_string(), "IMAGE".to_string(), "STATUS".to_string()],
        values: vec![
            vec!["my-pod".to_string(), "nginx:latest".to_string(), "Running".to_string()],
            vec!["other-pod".to_string(), "alpine:3".to_string(), "Pending".to_string()],
        ],
    }
}

#[test]
fn test_count() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COUNT", None), "2");
}

#[test]
fn test_col_substitution() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[IMAGE]", Some(0)), "nginx:latest");
    assert_eq!(ctx.resolve_arg("$COL[IMAGE]", Some(1)), "alpine:3");
}

#[test]
fn test_col_case_insensitive() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[image]", Some(0)), "nginx:latest");
    assert_eq!(ctx.resolve_arg("$COL[Image]", Some(0)), "nginx:latest");
}

#[test]
fn test_col_all_rows() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[NAME]", None), "my-pod,other-pod");
    assert_eq!(ctx.resolve_arg("$COL[IMAGE]", None), "nginx:latest,alpine:3");
}

#[test]
fn test_col_unknown_column_returns_empty() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[NONEXISTENT]", Some(0)), "");
    assert_eq!(ctx.resolve_arg("$COL[NONEXISTENT]", None), "");
}

#[test]
fn test_col_missing_closing_bracket_kept_literal() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[IMAGE", Some(0)), "$COL[IMAGE");
}

#[test]
fn test_col_out_of_bounds_row() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[IMAGE]", Some(99)), "");
}

#[test]
fn test_res_name_with_row_index() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[NAME]", Some(0)), "my-pod");
    assert_eq!(ctx.resolve_arg("$RES[NAME]", Some(1)), "other-pod");
}

#[test]
fn test_res_namespace_with_row_index() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[NAMESPACE]", Some(0)), "default");
    assert_eq!(ctx.resolve_arg("$RES[NAMESPACE]", Some(1)), "kube-system");
}

#[test]
fn test_res_uid_with_row_index() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[UID]", Some(0)), "uid-1234");
    assert_eq!(ctx.resolve_arg("$RES[UID]", Some(1)), "uid-5678");
}

#[test]
fn test_res_container_with_row_index() {
    let ctx = make_context();
    // second resource has no container, should return ""
    assert_eq!(ctx.resolve_arg("$RES[CONTAINER]", Some(0)), "main");
    assert_eq!(ctx.resolve_arg("$RES[CONTAINER]", Some(1)), "");
}

#[test]
fn test_res_all_rows_name() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[NAME]", None), "my-pod,other-pod");
}

#[test]
fn test_res_all_rows_namespace() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[NAMESPACE]", None), "default,kube-system");
}

#[test]
fn test_res_all_rows_uid() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[UID]", None), "uid-1234,uid-5678");
}

#[test]
fn test_res_all_rows_container_skips_none() {
    let ctx = make_context();
    // second resource has no container, filter_map skips it
    assert_eq!(ctx.resolve_arg("$RES[CONTAINER]", None), "main");
}

#[test]
fn test_res_case_insensitive() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[name]", Some(0)), "my-pod");
    assert_eq!(ctx.resolve_arg("$RES[Name]", Some(0)), "my-pod");
}

#[test]
fn test_res_unknown_field_returns_empty() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[UNKNOWN]", Some(0)), "");
    assert_eq!(ctx.resolve_arg("$RES[UNKNOWN]", None), "");
}

#[test]
fn test_res_out_of_bounds_row() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[NAME]", Some(99)), "");
}

#[test]
fn test_res_missing_closing_bracket_kept_literal() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$RES[NAME", Some(0)), "$RES[NAME");
}

#[test]
fn test_simple_placeholders() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$CONTEXT", Some(0)), "my-cluster");
    assert_eq!(ctx.resolve_arg("$NAMESPACE", Some(0)), "default");
    assert_eq!(ctx.resolve_arg("$PLURAL", Some(0)), "deployments");
    assert_eq!(ctx.resolve_arg("$GROUP", Some(0)), "apps");
    assert_eq!(ctx.resolve_arg("$VERSION", Some(0)), "v1");
}

#[test]
fn test_simple_placeholders_without_row_index() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$CONTEXT", None), "my-cluster");
    assert_eq!(ctx.resolve_arg("$NAMESPACE", None), "default");
}

#[test]
fn test_mixed_arg() {
    let ctx = make_context();
    assert_eq!(
        ctx.resolve_arg("$NAMESPACE/$COL[NAME]:$COL[STATUS]", Some(0)),
        "default/my-pod:Running"
    );
}

#[test]
fn test_mixed_res_and_col() {
    let ctx = make_context();
    assert_eq!(
        ctx.resolve_arg("$RES[NAME]/$RES[NAMESPACE]:$COL[STATUS]", Some(0)),
        "my-pod/default:Running"
    );
}

#[test]
fn test_no_placeholders() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("just a plain string", Some(0)), "just a plain string");
    assert_eq!(ctx.resolve_arg("", Some(0)), "");
    assert_eq!(ctx.resolve_arg("just a plain string", None), "just a plain string");
}

#[test]
fn test_unknown_placeholder_kept_literal() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$UNKNOWN", Some(0)), "$UNKNOWN");
    assert_eq!(ctx.resolve_arg("$UNKNOWN", None), "$UNKNOWN");
}

#[test]
fn test_dollar_sign_literal() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("costs $5 dollars", Some(0)), "costs $5 dollars");
}

#[test]
fn test_multiple_same_placeholder() {
    let ctx = make_context();
    assert_eq!(ctx.resolve_arg("$COL[NAME]-$COL[NAME]", Some(0)), "my-pod-my-pod");
    assert_eq!(ctx.resolve_arg("$RES[NAME]-$RES[NAME]", Some(0)), "my-pod-my-pod");
}
