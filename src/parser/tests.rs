use super::*;
use crate::lexer::lex;

fn parse(source: &str) -> Result<Vec<Expr>, ParseError> {
    let statements = parse_stmts(source)?;

    // the expression tests below only deal with expressions, so unwrap
    // each statement and collect the expression it carries
    let mut expressions = Vec::new();
    for statement in statements {
        match statement {
            Stmt::Expr(expr) => expressions.push(expr),
            stmt => panic!("expected an expression statement, got {stmt:?}"),
        }
    }

    Ok(expressions)
}

fn parse_stmts(source: &str) -> Result<Vec<Stmt>, ParseError> {
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

// mocking the span since it doesn't really matter here
fn identifier(name: &str) -> Identifier {
    Identifier {
        name: name.to_string(),
        span: (0..0).into(),
    }
}

fn assert_ident(identifier: &Identifier, name: &str, span: std::ops::Range<usize>) {
    assert_eq!(identifier.name, name);
    assert_eq!(identifier.span, span.into());
}

#[test]
fn parse_simple_and_nested_expressions() {
    use Expr::Integer as int;

    for (source, expected) in [
        ("1 + 2;", binary(BinaryOp::Add, int(1), int(2))),
        (
            "(1 + (2 + 3));",
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
            "1 + 2 * 3;",
            binary(Add, int(1), binary(Multiply, int(2), int(3))),
        ),
        (
            "(1 + 2) * 3;",
            binary(Multiply, binary(Add, int(1), int(2)), int(3)),
        ),
        (
            "8-3-1;",
            binary(Subtract, binary(Subtract, int(8), int(3)), int(1)),
        ),
        (
            "24 / 4 % 2 * 3;",
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
fn parse_logical_and_or_precedence() {
    // the && operator has higher precedence than ||
    use BinaryOp::*;
    use Expr::Integer as int;

    for (source, expected) in [
        // || is loosest, then &&, then the comparison operators
        (
            "1 < 2 && 3 < 4;",
            binary(
                And,
                binary(Less, int(1), int(2)),
                binary(Less, int(3), int(4)),
            ),
        ),
        (
            "1 || 2 && 3;",
            binary(Or, int(1), binary(And, int(2), int(3))),
        ),
        // both are left associative
        (
            "0 || 0 || 2;",
            binary(Or, binary(Or, int(0), int(0)), int(2)),
        ),
        (
            "1 && 2 && 3;",
            binary(And, binary(And, int(1), int(2)), int(3)),
        ),
    ] {
        assert_eq!(parse(source).unwrap(), vec![expected], "{source}");
    }
}

#[test]
fn parse_unary_signs() {
    assert_eq!(
        parse("-(1 + 2) * +3;").unwrap(),
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
    let source = "
int main() {
}";
    assert_eq!(parse(source).unwrap(), vec![]);
    let source = "
int main() {
    (1 + (2 + 3));
    4 * 5;
}";
    assert_eq!(
        parse(source).unwrap(),
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
fn parse_return_statements() {
    let source = "
int main() {
    return 42;
}";
    assert_eq!(
        parse_stmts(source).unwrap(),
        vec![Stmt::Return(Expr::Integer(42))]
    );
    assert_eq!(
        parse_stmts("return 1 + 2 * 3;").unwrap(),
        vec![Stmt::Return(binary(
            BinaryOp::Add,
            Expr::Integer(1),
            binary(BinaryOp::Multiply, Expr::Integer(2), Expr::Integer(3)),
        ))]
    );
    let source = "
int main() {
    1;
    return 2;
}";
    assert_eq!(
        parse_stmts(source).unwrap(),
        vec![Stmt::Expr(Expr::Integer(1)), Stmt::Return(Expr::Integer(2))]
    );
}

#[test]
fn parse_declarations() {
    let source = "
int main() {
    int x;
}";
    let expected = vec![Stmt::Decl(identifier("x"))];
    assert_eq!(parse_stmts(source).unwrap(), expected);

    let source = "
int main() {
    int x;
    int y;
}";
    let expected = vec![Stmt::Decl(identifier("x")), Stmt::Decl(identifier("y"))];
    assert_eq!(parse_stmts(source).unwrap(), expected);

    // declarations coexist with expression and return statements
    let source = "
int main() {
    int x;
    1;
    return 2;
}";
    let expected = vec![
        Stmt::Decl(identifier("x")),
        Stmt::Expr(Expr::Integer(1)),
        Stmt::Return(Expr::Integer(2)),
    ];
    assert_eq!(parse_stmts(source).unwrap(), expected);
}

#[test]
fn parse_declarations_with_initializers() {
    let source = "
int main() {
    int x = 1 + 2 * 3;
}";
    let expected = vec![Stmt::DeclAssignment(
        identifier("x"),
        binary(
            BinaryOp::Add,
            Expr::Integer(1),
            binary(BinaryOp::Multiply, Expr::Integer(2), Expr::Integer(3)),
        ),
    )];
    assert_eq!(parse_stmts(source).unwrap(), expected);
}

#[test]
fn parse_assignments_and_variable_reads() {
    // the rhs of an assignment is a full expression
    let source = "
int main() {
    int x;
    x = 1 + 2 * 3;
}";
    assert_eq!(
        parse_stmts(source).unwrap(),
        vec![
            Stmt::Decl(identifier("x")),
            Stmt::Assign(
                identifier("x"),
                binary(
                    BinaryOp::Add,
                    Expr::Integer(1),
                    binary(BinaryOp::Multiply, Expr::Integer(2), Expr::Integer(3)),
                )
            ),
        ]
    );

    // variables are ordinary expressions, e.g. in return and binary exprs
    let source = "
int main() {
    int x;
    x = 5;
    return x + 1;
}";
    assert_eq!(
        parse_stmts(source).unwrap(),
        vec![
            Stmt::Decl(identifier("x")),
            Stmt::Assign(identifier("x"), Expr::Integer(5)),
            Stmt::Return(binary(
                BinaryOp::Add,
                Expr::Variable(identifier("x")),
                Expr::Integer(1),
            )),
        ]
    );

    // a bare identifier is a discarded variable read
    let source = "
int main() {
    x;
}";
    assert_eq!(
        parse_stmts(source).unwrap(),
        vec![Stmt::Expr(Expr::Variable(identifier("x")))]
    );
}

#[test]
fn identifiers_preserve_their_source_span() {
    let source = "
int main() {
    int x;
    x = 5;
    return x + 1;
}";
    let statements = parse_stmts(source).unwrap();
    match statements.as_slice() {
        [
            Stmt::Decl(x),
            Stmt::Assign(y, _),
            Stmt::Return(Expr::Binary { lhs, .. }),
        ] => {
            assert_ident(x, "x", 22..23);
            assert_ident(y, "x", 29..30);
            match lhs.as_ref() {
                Expr::Variable(z) => assert_ident(z, "x", 47..48),
                other => panic!("expected a variable read, got {other:?}"),
            }
        }
        other => panic!("unexpected statements: {other:?}"),
    }
}

#[test]
fn reject_malformed_input_with_expected_token_and_span() {
    for (source, expected, span) in [
        ("1 + * 2", "an integer, identifier or '('", 4..5),
        ("(1 + 2;", "')'", 6..7),
        (")", "an integer, identifier or '('", 0..1),
        // statements now own their terminator, so a missing ';' is reported
        // where the next token starts instead of as trailing garbage
        ("1 2", "';'", 2..3),
        ("1)", "';'", 1..2),
        ("1++2", "';'", 1..3),
        ("1--2", "';'", 1..3),
        // intmain is now a valid (undeclared) identifier, so the error moves
        // to the '(' that no longer continues the statement
        ("intmain() {}", "';'", 7..8),
        ("int main2() {}", "'main'", 4..9),
        ("int main(1) {}", "')'", 9..10),
        ("int main() { 1 }", "';'", 15..16),
        ("int main() {} 1", "end of input", 14..15),
        ("int main() { int; }", "an identifier after 'int'", 16..17),
        ("int main() { int 5; }", "an identifier after 'int'", 17..18),
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
        ("", "an integer, identifier or '('"),
        ("1 +", "an integer, identifier or '('"),
        ("(1 + 2", "')'"),
        ("int main() {", "a statement or '}'"),
        ("int main() { 1;", "a statement or '}'"),
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
        parse("9223372036854775807;").unwrap(),
        vec![Expr::Integer(i64::MAX)]
    );
    assert_eq!(
        parse("-9223372036854775807 - 1;").unwrap(),
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
    let source = "1 + 2;";
    let mut tokens = lex(source).unwrap();
    tokens[2].kind = TokenKind::Integer(3);

    assert_eq!(
        Parser::new(tokens, source).parse_program().unwrap(),
        vec![Stmt::Expr(binary(
            BinaryOp::Add,
            Expr::Integer(1),
            Expr::Integer(3),
        ))]
    );
}

#[test]
fn parse_if_else_statements() {
    let source = "
int main() {
    if (1) x = 2;
    if (x) y = 1; else y = 2;
}";
    assert_eq!(
        parse_stmts(source).unwrap(),
        vec![
            Stmt::If {
                condition: Expr::Integer(1),
                then: Box::new(Stmt::Assign(identifier("x"), Expr::Integer(2))),
                else_stmt: None,
            },
            Stmt::If {
                condition: Expr::Variable(identifier("x")),
                then: Box::new(Stmt::Assign(identifier("y"), Expr::Integer(1))),
                else_stmt: Some(Box::new(Stmt::Assign(identifier("y"), Expr::Integer(2)))),
            },
        ]
    );
}

#[test]
fn else_binds_to_the_nearest_unmatched_if() {
    let source = "
int main() {
    if (a)
    if (b) x = 1; else x = 2;
}";
    assert_eq!(
        parse_stmts(source).unwrap(),
        vec![Stmt::If {
            condition: Expr::Variable(identifier("a")),
            then: Box::new(Stmt::If {
                condition: Expr::Variable(identifier("b")),
                then: Box::new(Stmt::Assign(identifier("x"), Expr::Integer(1))),
                else_stmt: Some(Box::new(Stmt::Assign(identifier("x"), Expr::Integer(2)))),
            }),
            else_stmt: None,
        }]
    );
}

#[test]
fn if_condition_must_be_parenthesized() {
    let source = "int main() { if x) y = 1; }";
    let error = parse_stmts(source).unwrap_err();
    assert!(
        matches!(
            &error,
            ParseError::UnexpectedToken {
                span,
                expected,
                ..
            } if span.offset() == 16 && span.len() == 1 && expected == "'('"
        ),
        "{error:?}"
    );
}

#[test]
fn if_body_may_not_be_empty() {
    let source = "int main() { if (x) }";
    let error = parse_stmts(source).unwrap_err();
    assert!(
        matches!(
            &error,
            ParseError::UnexpectedToken {
                span,
                expected,
                ..
            } if span.offset() == 20 && span.len() == 1
                && expected == "an integer, identifier or '('"
        ),
        "{error:?}"
    );
}
