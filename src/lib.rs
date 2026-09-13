mod errors;
mod lexer;
mod parser;

pub use errors::{AppError, LexError, ParseError};
pub use lexer::{Token, TokenKind};
pub use parser::{BinaryOp, Expr, UnaryOp};

pub fn lex(source: &str) -> miette::Result<Vec<Token>> {
    lexer::lex(source)
        .map_err(AppError::from)
        .map_err(miette::Report::new)
}

pub fn parse(tokens: Vec<Token>, source: &str) -> miette::Result<Vec<Expr>> {
    parser::Parser::new(tokens, source)
        .parse_program()
        .map_err(AppError::from)
        .map_err(miette::Report::new)
}
