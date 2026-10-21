use logos::Logos;
use miette::SourceSpan;

use crate::errors::LexError;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

#[derive(Logos, Debug, PartialEq, Eq, Clone)]
#[logos(skip r"[ \t\r\n\f\x0b]+")]
pub enum TokenKind {
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(";")]
    Semicolon,

    #[token("int")]
    Int,
    #[token("main")]
    Main,

    // Consume whole identifiers so `intmain` cannot become `int main`.
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

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
