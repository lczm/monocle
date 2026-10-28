use logos::Logos;
use miette::SourceSpan;

use crate::errors::LexError;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

#[derive(Logos, Debug, PartialEq, Eq, Clone)]
#[logos(skip(r"[ \t\r\n\f\x0b]+|//[^\n]*", allow_greedy = true))]
pub enum TokenKind {
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,

    #[token("int")]
    Int,
    #[token("sizeof")]
    Sizeof,
    // todo : this has to be removed in the future when
    // the function names can be parsed
    #[token("main")]
    Main,
    #[token("return")]
    Return,

    // control flow
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,

    // Consume whole identifiers so `intmain` cannot become `int main`.
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Arithmetic
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    // Recognize these as single C tokens; parsing them is not supported yet.
    #[token("++")]
    Increment,
    #[token("--")]
    Decrement,

    // Comparison
    #[token("==")]
    EqualEqual,
    #[token("!=")]
    BangEqual,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,

    // Logical
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("!")]
    Bang,

    // Bitwise and shifts
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("<<")]
    LessLess,
    #[token(">>")]
    GreaterGreater,

    // Shortcut assignments
    #[token("=")]
    Equal,
    #[token("+=")]
    PlusEqual,
    #[token("-=")]
    MinusEqual,
    #[token("*=")]
    StarEqual,
    #[token("/=")]
    SlashEqual,
    #[token("%=")]
    PercentEqual,
    #[token("&=")]
    AmpersandEqual,
    #[token("|=")]
    PipeEqual,
    #[token("^=")]
    CaretEqual,
    #[token("<<=")]
    LessLessEqual,
    #[token(">>=")]
    GreaterGreaterEqual,

    // Ternary and member access
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,

    // Signs are operators so subtraction also works without whitespace: 1-2.
    // lex them as i128 instead of i64 so that we can detect overflow
    // in the parser and report it as IntegerOverflow
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i128>().ok())]
    Integer(i128),
}

pub(crate) fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    TokenKind::lexer(source)
        .spanned()
        .map(|(kind, span)| match kind {
            Ok(kind) => Ok(Token {
                kind,
                span: span.into(),
            }),
            Err(_) => Err(LexError::new(source, span)),
        })
        .collect()
}

#[cfg(test)]
mod tests;
