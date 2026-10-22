use std::{iter::Peekable, vec::IntoIter};

use miette::NamedSource;

use crate::{
    errors::ParseError,
    lexer::{Token, TokenKind},
};

#[derive(Debug, PartialEq)]
pub enum Expr {
    Integer(i64),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

pub(crate) struct Parser<'a> {
    iter: Peekable<IntoIter<Token>>,
    src: &'a str,
}

// the lower the function in the call tree, the higher precedence it has
// for example : additive = multiplicative (("+" | "-") multiplicative)*
// just says that multiplicative has a higher precedence
impl<'a> Parser<'a> {
    pub(crate) fn new(tokens: Vec<Token>, source: &'a str) -> Self {
        Self {
            iter: tokens.into_iter().peekable(),
            src: source,
        }
    }

    pub(crate) fn parse_program(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut expressions = Vec::new();

        // hard code the int main
        if self.peek() == Some(TokenKind::Int) {
            self.expect(TokenKind::Int, "'int'")?;
            self.expect(TokenKind::Main, "'main'")?;
            self.expect(TokenKind::LParen, "'('")?;
            self.expect(TokenKind::RParen, "')'")?;
            self.expect(TokenKind::LBrace, "'{'")?;

            while self.peek() != Some(TokenKind::RBrace) {
                if self.peek().is_none() {
                    return Err(self.expected_error("an expression or '}'"));
                }
                expressions.push(self.parse_expr()?);
                self.expect(TokenKind::Semicolon, "';'")?;
            }
            self.expect(TokenKind::RBrace, "'}'")?;
        } else {
            expressions.push(self.parse_expr()?);
            if self.peek() == Some(TokenKind::Semicolon) {
                self.iter.next();
            }
        }

        if self.peek().is_some() {
            return Err(self.expected_error("end of input"));
        }

        Ok(expressions)
    }

    pub(crate) fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_equality()
    }

    // equality = relational (("==" | "!=") relational)*
    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_relational()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::EqualEqual) => BinaryOp::Equal,
                Some(TokenKind::BangEqual) => BinaryOp::NotEqual,
                _ => break,
            };
            self.iter.next();
            let rhs = self.parse_relational()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // relational = additive (("<" | "<=" | ">" | ">=") additive)*
    fn parse_relational(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Less) => BinaryOp::Less,
                Some(TokenKind::LessEqual) => BinaryOp::LessEqual,
                Some(TokenKind::Greater) => BinaryOp::Greater,
                Some(TokenKind::GreaterEqual) => BinaryOp::GreaterEqual,
                _ => break,
            };
            self.iter.next();
            let rhs = self.parse_additive()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // additive = multiplicative (("+" | "-") multiplicative)*
    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Plus) => BinaryOp::Add,
                Some(TokenKind::Minus) => BinaryOp::Subtract,
                _ => break,
            };
            self.iter.next();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // multiplicative = unary (("*" | "/" | "%") unary)*
    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Star) => BinaryOp::Multiply,
                Some(TokenKind::Slash) => BinaryOp::Divide,
                Some(TokenKind::Percent) => BinaryOp::Modulo,
                _ => break,
            };
            self.iter.next();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // unary = ("+" | "-") unary | primary
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        let op = match self.peek() {
            Some(TokenKind::Plus) => UnaryOp::Plus,
            Some(TokenKind::Minus) => UnaryOp::Minus,
            _ => return self.parse_primary(),
        };
        self.iter.next();
        Ok(Expr::Unary {
            op,
            operand: Box::new(self.parse_unary()?),
        })
    }

    // primary = integer | "(" expression ")"
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Some(TokenKind::Integer(value)) => {
                let span = self.iter.peek().unwrap().span;
                self.iter.next();
                match i64::try_from(value) {
                    Ok(value) => Ok(Expr::Integer(value)),
                    Err(_) => Err(ParseError::IntegerOverflow {
                        src: NamedSource::new("input", self.src.to_string()),
                        span,
                        value,
                    }),
                }
            }
            Some(TokenKind::LParen) => {
                self.iter.next();
                let expression = self.parse_expr()?;
                self.expect(TokenKind::RParen, "')'")?;
                Ok(expression)
            }
            _ => Err(self.expected_error("an integer or '('")),
        }
    }

    fn peek(&mut self) -> Option<TokenKind> {
        self.iter.peek().map(|token| token.kind.clone())
    }

    fn expect(&mut self, token: TokenKind, expected: &str) -> Result<(), ParseError> {
        if self.peek() == Some(token) {
            self.iter.next();
            Ok(())
        } else {
            Err(self.expected_error(expected))
        }
    }

    fn expected_error(&mut self, expected: &str) -> ParseError {
        let src = NamedSource::new("input", self.src.to_string());
        let expected = expected.to_string();
        match self.iter.peek() {
            Some(token) => ParseError::UnexpectedToken {
                src,
                span: token.span,
                expected,
            },
            None => ParseError::UnexpectedEOF {
                src,
                // A zero-width label marks EOF, including empty input.
                span: (self.src.len(), 0).into(),
                expected,
            },
        }
    }
}

#[cfg(test)]
mod tests;
