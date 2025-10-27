use super::*;

#[test]
fn evaluate_test() {
    let name = "a".to_owned();
    let labels = ["1: e".to_owned(), "2: f".to_owned()];
    let annotations = ["1: g".to_owned(), "2: h".to_owned(), "3: i".to_owned()];

    let mut statements = Vec::new();
    statements.push(&name);
    statements.extend(labels.iter());
    statements.extend(annotations.iter());

    // should match
    let to_check = [
        "a",
        "i",
        "a & f",
        "a | f",
        "!(b | c)",
        "a & (b | h)",
        "!((a & c) | (e & z))",
        "!(!(!((a & c) | (e & z))))",
    ];
    for expression in &to_check {
        assert!(statements.evaluate(&parse(expression).unwrap()), "should match: {expression}");
    }

    // shouldn't match
    let to_check = [
        "b",
        "!i",
        "!(a | b)",
        "a & (b | c)",
        "(a & c) | (e & z)",
        "!(!((a & c) | (e & z)))",
    ];
    for expression in &to_check {
        assert!(
            !statements.evaluate(&parse(expression).unwrap()),
            "shouldn't match: {expression}"
        );
    }
}

#[test]
#[should_panic(expected = "ExpectedOperator(2)")]
fn expected_operator_test() {
    parse("a ( b & c )").unwrap();
}

#[test]
#[should_panic(expected = "ExpectedOperator(6)")]
fn expected_operator_2_test() {
    parse("( a ) ( b & c )").unwrap();
}

#[test]
#[should_panic(expected = "ExpectedOperator(14)")]
fn expected_operator_3_test() {
    parse("a & ( b & c ) d").unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedOperator(4)")]
fn unexpected_operator_test() {
    parse("a & & b").unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedOperator(2)")]
fn unexpected_operator_2_test() {
    parse("! ! a").unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedOperator(6)")]
fn unexpected_operator_3_test() {
    parse("a & ! & a").unwrap();
}

#[test]
#[should_panic(expected = "ExpectedOperator(10)")]
fn not_operator_test() {
    parse("( a & b ) ! b").unwrap();
}

#[test]
#[should_panic(expected = "ExpectedOperator(2)")]
fn not_operator_2_test() {
    parse("!a!bc").unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedClosingBracket(2)")]
fn no_opening_bracket_test() {
    parse("a ) ( b & c )").unwrap();
}

#[test]
#[should_panic(expected = "ExpectedClosingBracket(16)")]
fn no_closing_bracket_test() {
    parse("( a & b ) & c & ( ( d & e )").unwrap();
}
