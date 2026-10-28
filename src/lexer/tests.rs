use super::*;

fn token_kinds(tokens: Vec<Token>) -> Vec<TokenKind> {
    tokens.into_iter().map(|token| token.kind).collect()
}

#[test]
fn lex_nested_expression() {
    assert_eq!(
        token_kinds(
            lex(r#"
(1 + (2 + 3))
"#)
            .unwrap()
        ),
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
        token_kinds(
            lex(r#"
1-2 + -3 * 4 / 5 % 6
"#)
            .unwrap()
        ),
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
        token_kinds(
            lex(r#"
int main() {
	return 1 + 2;
}
"#)
            .unwrap()
        ),
        vec![
            TokenKind::Int,
            TokenKind::Main,
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Return,
            TokenKind::Integer(1),
            TokenKind::Plus,
            TokenKind::Integer(2),
            TokenKind::Semicolon,
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn skip_line_comments() {
    assert_eq!(
        token_kinds(
            lex(r#"
1 // comment with int main() inside
+ 2
"#)
            .unwrap()
        ),
        vec![
            TokenKind::Integer(1),
            TokenKind::Plus,
            TokenKind::Integer(2),
        ]
    );
}

#[test]
fn line_comment_at_end_of_input_and_division_still_works() {
    assert_eq!(
        token_kinds(lex(r#"1//no trailing newline"#).unwrap()),
        vec![TokenKind::Integer(1)]
    );
    assert_eq!(
        token_kinds(
            lex(r#"
1 / 2 3//x
4
"#)
            .unwrap()
        ),
        vec![
            TokenKind::Integer(1),
            TokenKind::Slash,
            TokenKind::Integer(2),
            TokenKind::Integer(3),
            TokenKind::Integer(4),
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
fn lex_every_operator() {
    assert_eq!(
        token_kinds(
            lex("a sizeof(int) x->y z[1] == != <= >= < > && || ! & | ^ ~ \
                 << >> = += -= *= /= %= &= |= ^= <<= >>= ? : . ,")
            .unwrap()
        ),
        vec![
            TokenKind::Identifier("a".into()),
            TokenKind::Sizeof,
            TokenKind::LParen,
            TokenKind::Int,
            TokenKind::RParen,
            TokenKind::Identifier("x".into()),
            TokenKind::Arrow,
            TokenKind::Identifier("y".into()),
            TokenKind::Identifier("z".into()),
            TokenKind::LBracket,
            TokenKind::Integer(1),
            TokenKind::RBracket,
            TokenKind::EqualEqual,
            TokenKind::BangEqual,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::AndAnd,
            TokenKind::OrOr,
            TokenKind::Bang,
            TokenKind::Ampersand,
            TokenKind::Pipe,
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::LessLess,
            TokenKind::GreaterGreater,
            TokenKind::Equal,
            TokenKind::PlusEqual,
            TokenKind::MinusEqual,
            TokenKind::StarEqual,
            TokenKind::SlashEqual,
            TokenKind::PercentEqual,
            TokenKind::AmpersandEqual,
            TokenKind::PipeEqual,
            TokenKind::CaretEqual,
            TokenKind::LessLessEqual,
            TokenKind::GreaterGreaterEqual,
            TokenKind::Question,
            TokenKind::Colon,
            TokenKind::Dot,
            TokenKind::Comma,
        ]
    );
}

#[test]
fn lex_operators_without_whitespace_longest_match() {
    assert_eq!(
        token_kinds(lex("x==1x!=2x<=3x>=4x&&y x||y x->y a->b 1--2").unwrap()),
        vec![
            TokenKind::Identifier("x".into()),
            TokenKind::EqualEqual,
            TokenKind::Integer(1),
            TokenKind::Identifier("x".into()),
            TokenKind::BangEqual,
            TokenKind::Integer(2),
            TokenKind::Identifier("x".into()),
            TokenKind::LessEqual,
            TokenKind::Integer(3),
            TokenKind::Identifier("x".into()),
            TokenKind::GreaterEqual,
            TokenKind::Integer(4),
            TokenKind::Identifier("x".into()),
            TokenKind::AndAnd,
            TokenKind::Identifier("y".into()),
            TokenKind::Identifier("x".into()),
            TokenKind::OrOr,
            TokenKind::Identifier("y".into()),
            TokenKind::Identifier("x".into()),
            TokenKind::Arrow,
            TokenKind::Identifier("y".into()),
            TokenKind::Identifier("a".into()),
            TokenKind::Arrow,
            TokenKind::Identifier("b".into()),
            TokenKind::Integer(1),
            TokenKind::Decrement,
            TokenKind::Integer(2),
        ]
    );
}

#[test]
fn report_invalid_tokens_with_spans() {
    for (source, span) in [
        ("@", 0..1),
        ("1 + @", 4..5),
        ("1 @", 2..3),
        ("int @", 4..5),
        ("int main() {} @", 14..15),
        ("1 + 🦀", 4..8),
        // more than i128, it is a lex error
        ("999999999999999999999999999999999999999", 0..39),
    ] {
        let error = lex(source).unwrap_err();
        assert_eq!(error.src.inner(), source);
        assert_eq!(error.bad_token_span, span.into());
    }
}

#[test]
fn lex_control_flow_keywords() {
    assert_eq!(
        token_kinds(
            lex(r#"
if else while for break continue
"#)
            .unwrap()
        ),
        vec![
            TokenKind::If,
            TokenKind::Else,
            TokenKind::While,
            TokenKind::For,
            TokenKind::Break,
            TokenKind::Continue,
        ]
    );
}

#[test]
fn lex_i64_min_magnitude_and_i64_max() {
    assert_eq!(
        token_kinds(lex("-9223372036854775808").unwrap()),
        vec![
            TokenKind::Minus,
            TokenKind::Integer(9223372036854775808_i128),
        ]
    );
    assert_eq!(
        token_kinds(lex("9223372036854775807").unwrap()),
        vec![TokenKind::Integer(9223372036854775807_i128)]
    );
}
