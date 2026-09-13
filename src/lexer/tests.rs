use super::*;

fn token_kinds(tokens: Vec<Token>) -> Vec<TokenKind> {
    tokens.into_iter().map(|token| token.kind).collect()
}

#[test]
fn lex_nested_expression() {
    assert_eq!(
        token_kinds(lex("(1 + (2 + 3))").unwrap()),
        vec![
            TokenKind::LParen,
            TokenKind::Integer(1),
            TokenKind::Plus,
            TokenKind::LParen,
            TokenKind::Integer(2),
            TokenKind::Plus,
            TokenKind::Integer(3),
            TokenKind::RParen,
            TokenKind::RParen,
        ]
    );
}

#[test]
fn lex_arithmetic_and_separate_signs() {
    assert_eq!(
        token_kinds(lex("1-2 + -3 * 4 / 5 % 6").unwrap()),
        vec![
            TokenKind::Integer(1),
            TokenKind::Minus,
            TokenKind::Integer(2),
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Integer(3),
            TokenKind::Star,
            TokenKind::Integer(4),
            TokenKind::Slash,
            TokenKind::Integer(5),
            TokenKind::Percent,
            TokenKind::Integer(6),
        ]
    );
}

#[test]
fn lex_wrapper_and_skip_whitespace() {
    assert_eq!(
        token_kinds(lex("int main() {\r\n\t1 + 2;\n}\x0c\x0b").unwrap()),
        vec![
            TokenKind::Int,
            TokenKind::Main,
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Integer(1),
            TokenKind::Plus,
            TokenKind::Integer(2),
            TokenKind::Semicolon,
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn preserve_identifier_boundaries_and_compound_operators() {
    assert_eq!(
        token_kinds(lex("intmain main2 1++2 1--2").unwrap()),
        vec![
            TokenKind::Identifier("intmain".into()),
            TokenKind::Identifier("main2".into()),
            TokenKind::Integer(1),
            TokenKind::Increment,
            TokenKind::Integer(2),
            TokenKind::Integer(1),
            TokenKind::Decrement,
            TokenKind::Integer(2),
        ]
    );
}

#[test]
fn report_invalid_tokens_and_overflow_with_spans() {
    for (source, span) in [
        ("@", 0..1),
        ("1 + @", 4..5),
        ("1 @", 2..3),
        ("int @", 4..5),
        ("int main() {} @", 14..15),
        ("1 + 🦀", 4..8),
        ("9223372036854775808", 0..19),
    ] {
        let error = lex(source).unwrap_err();
        assert_eq!(error.src.inner(), source);
        assert_eq!(error.bad_token_span, span.into());
    }
}
