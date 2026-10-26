mod codegen;
mod errors;
mod lexer;
mod parser;

pub use codegen::codegen;
pub use errors::{AppError, CodegenError, LexError, ParseError};
pub use lexer::{Token, TokenKind};
pub use parser::{BinaryOp, Expr, Stmt, UnaryOp};

pub fn lex(source: &str) -> Result<Vec<Token>, AppError> {
    lexer::lex(source).map_err(|e| AppError::from(Box::new(e)))
}

pub fn parse(tokens: Vec<Token>, source: &str) -> Result<Vec<Stmt>, AppError> {
    parser::Parser::new(tokens, source)
        .parse_program()
        .map_err(|e| AppError::from(Box::new(e)))
}
