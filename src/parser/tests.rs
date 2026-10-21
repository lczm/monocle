use super::*;
use crate::lexer::lex;

fn parse(source: &str) -> Result<Vec<Expr>, ParseError> {
    let tokens = lex(source).unwrap();
    Parser::new(tokens, source).parse_program()
}

fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}

#[test]
fn parse_simple_and_nested_expressions() {
    use Expr::Integer as int;

    for (source, expected) in [
        ("1 + 2", binary(BinaryOp::Add, int(1), int(2))),
        (
            "(1 + (2 + 3))",
            binary(BinaryOp::Add, int(1), binary(BinaryOp::Add, int(2), int(3))),
        ),
    ] {
        assert_eq!(parse(source).unwrap(), vec![expected]);
    }
}

#[test]
fn parse_precedence_and_left_associativity() {
    use BinaryOp::*;
    use Expr::Integer as int;

    for (source, expected) in [
        (
            "1 + 2 * 3",
            binary(Add, int(1), binary(Multiply, int(2), int(3))),
        ),
        (
            "(1 + 2) * 3",
            binary(Multiply, binary(Add, int(1), int(2)), int(3)),
        ),
        (
            "8-3-1",
            binary(Subtract, binary(Subtract, int(8), int(3)), int(1)),
        ),
        (
            "24 / 4 % 2 * 3",
            binary(
                Multiply,
                binary(Modulo, binary(Divide, int(24), int(4)), int(2)),
                int(3),
            ),
        ),
    ] {
        assert_eq!(parse(source).unwrap(), vec![expected], "{source}");
    }
}

#[test]
fn parse_unary_signs() {
    assert_eq!(
        parse("-(1 + 2) * +3").unwrap(),
        vec![binary(
            BinaryOp::Multiply,
            Expr::Unary {
                op: UnaryOp::Minus,
                operand: Box::new(binary(BinaryOp::Add, Expr::Integer(1), Expr::Integer(2))),
            },
            Expr::Unary {
                op: UnaryOp::Plus,
                operand: Box::new(Expr::Integer(3)),
            },
        )]
    );
}

#[test]
fn wrapper_only_groups_expressions() {
    assert_eq!(parse("int main() { }").unwrap(), vec![]);
    assert_eq!(
        parse("int main() { (1 + (2 + 3)); 4 * 5; }").unwrap(),
        vec![
            binary(
                BinaryOp::Add,
                Expr::Integer(1),
                binary(BinaryOp::Add, Expr::Integer(2), Expr::Integer(3)),
            ),
            binary(BinaryOp::Multiply, Expr::Integer(4), Expr::Integer(5)),
        ]
    );
    assert_eq!(parse("42;").unwrap(), vec![Expr::Integer(42)]);
}

#[test]
fn reject_malformed_input_with_expected_token_and_span() {
    for (source, expected, span) in [
        ("1 + * 2", "an integer or '('", 4..5),
        ("(1 + 2;", "')'", 6..7),
        (")", "an integer or '('", 0..1),
        ("1 2", "end of input", 2..3),
        ("1)", "end of input", 1..2),
        ("1++2", "end of input", 1..3),
        ("1--2", "end of input", 1..3),
        ("intmain() {}", "an integer or '('", 0..7),
        ("int main2() {}", "'main'", 4..9),
        ("int main(1) {}", "')'", 9..10),
        ("int main() { 1 }", "';'", 15..16),
        ("int main() {} 1", "end of input", 14..15),
    ] {
        let error = parse(source).unwrap_err();
        assert!(
            matches!(
                error,
                ParseError::UnexpectedToken {
                    span: actual_span,
                    expected: actual_expected,
                    ..
                } if actual_span == span.into() && actual_expected == expected
            ),
            "{source}"
        );
    }
}

#[test]
fn report_missing_input_at_eof() {
    for (source, expected) in [
        ("", "an integer or '('"),
        ("1 +", "an integer or '('"),
        ("(1 + 2", "')'"),
        ("int main() {", "an expression or '}'"),
        ("int main() { 1;", "an expression or '}'"),
    ] {
        let error = parse(source).unwrap_err();
        assert!(
            matches!(
                error,
                ParseError::UnexpectedEOF {
                    span,
                    expected: actual_expected,
                    ..
                } if span.offset() == source.len()
                    && span.is_empty()
                    && actual_expected == expected
            ),
            "{source}"
        );
    }
}

#[test]
fn parse_i64_max_literal_and_limits_style_min() {
    // the i64::MIN -9223372036854775808 is represented as
    // -9223372036854775807 - 1 in limits.h
    assert_eq!(
        parse("9223372036854775807").unwrap(),
        vec![Expr::Integer(i64::MAX)]
    );
    assert_eq!(
        parse("-9223372036854775807 - 1").unwrap(),
        vec![binary(
            BinaryOp::Subtract,
            Expr::Unary {
                op: UnaryOp::Minus,
                operand: Box::new(Expr::Integer(i64::MAX)),
            },
            Expr::Integer(1),
        )]
    );
}

#[test]
fn reject_integer_literals_outside_i64_range() {
    for (source, span, value) in [
        ("9223372036854775808", 0..19, 9223372036854775808_i128),
        ("-9223372036854775808", 1..20, 9223372036854775808_i128),
        ("-9223372036854775809", 1..20, 9223372036854775809_i128),
        ("1 - 9223372036854775808", 4..23, 9223372036854775808_i128),
    ] {
        let error = parse(source).unwrap_err();
        assert!(
            matches!(
                error,
                ParseError::IntegerOverflow {
                    span: actual_span,
                    value: actual_value,
                    ..
                } if actual_span == span.into() && actual_value == value
            ),
            "{source}"
        );
    }
}

#[test]
fn parse_supplied_tokens() {
    let source = "1 + 2";
    let mut tokens = lex(source).unwrap();
    tokens[2].kind = TokenKind::Integer(3);

    assert_eq!(
        Parser::new(tokens, source).parse_program().unwrap(),
        vec![binary(BinaryOp::Add, Expr::Integer(1), Expr::Integer(3))]
    );
}
