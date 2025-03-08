use super::*;

#[test]
fn simple_and_test() {
    let actual = expand("a + b").unwrap();
    assert_eq!(vec![vec!["a", "b"]], actual);

    let actual = expand("a + b + c").unwrap();
    assert_eq!(vec![vec!["a", "b", "c"]], actual);
}

#[test]
fn simple_or_test() {
    let actual = expand("a | b").unwrap();
    assert_eq!(vec![vec!["a"], vec!["b"]], actual);

    let actual = expand("a | b | c").unwrap();
    assert_eq!(vec![vec!["a"], vec!["b"], vec!["c"]], actual);
}

#[test]
fn brackets_test() {
    let actual = expand("a + ( b | c )").unwrap();
    assert_eq!(vec![vec!["a", "b"], vec!["a", "c"]], actual);

    let actual = expand("a | ( b + c )").unwrap();
    assert_eq!(vec![vec!["a"], vec!["b", "c"]], actual);

    let actual = expand("( a + b ) | c").unwrap();
    assert_eq!(vec![vec!["a", "b"], vec!["c"]], actual);

    let actual = expand("( a | b ) + c").unwrap();
    assert_eq!(vec![vec!["a", "c"], vec!["b", "c"]], actual);
}

#[test]
#[should_panic(expected = "ExpectedValue(4)")]
fn expected_value_test() {
    expand("a + + b").unwrap();
}

#[test]
#[should_panic(expected = "ExpectedOperator(2)")]
fn expected_operator_test() {
    expand("a ( b + c )").unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedClosingBracket(2)")]
fn no_opening_bracket_test() {
    expand("a ) ( b + c )").unwrap();
}

#[test]
#[should_panic(expected = "ExpectedClosingBracket(16)")]
fn no_closing_bracket_test() {
    expand("( a + b ) + c + ( ( d + e )").unwrap();
}
