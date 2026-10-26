use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum AppError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Lexer(#[from] Box<LexError>),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Parser(#[from] Box<ParseError>),
}

#[derive(Error, Debug, Diagnostic)]
#[error("Invalid token or out-of-range integer")]
#[diagnostic(
    code(monocle::lex_error),
    help(
        "Use decimal integers in the i64 range, arithmetic operators, parentheses, or the int main() wrapper"
    )
)]
pub struct LexError {
    #[source_code]
    pub src: NamedSource<String>,

    #[label("Cannot lex this token")]
    pub bad_token_span: SourceSpan,
}

impl LexError {
    pub(crate) fn new(source: &str, span: std::ops::Range<usize>) -> Self {
        Self {
            src: NamedSource::new("input", source.to_string()),
            bad_token_span: span.into(),
        }
    }
}

#[derive(Error, Debug, Diagnostic, PartialEq)]
#[error("undefined variable `{name}`")]
#[diagnostic(
    code(monocle::undefined_variable),
    help("Declare the variable before using it")
)]
pub struct CodegenError {
    pub name: String,
}

#[derive(Error, Debug, Diagnostic, PartialEq)]
pub enum ParseError {
    #[error("Unexpected token")]
    #[diagnostic(code(monocle::unexpected_token))]
    UnexpectedToken {
        #[source_code]
        src: NamedSource<String>,

        #[label("Expected {expected}")]
        span: SourceSpan,
        expected: String,
    },

    #[error("Unexpected end of file: expected {expected}")]
    #[diagnostic(code(monocle::unexpected_eof))]
    UnexpectedEOF {
        #[source_code]
        src: NamedSource<String>,

        #[label("Expected {expected}")]
        span: SourceSpan,
        expected: String,
    },

    #[error("Integer literal out of i64 range: {value}")]
    #[diagnostic(code(monocle::integer_overflow))]
    IntegerOverflow {
        #[source_code]
        src: NamedSource<String>,

        #[label("This literal doesn't fit in i64")]
        span: SourceSpan,
        value: i128,
    },
}
