use std::{iter::Peekable, vec::IntoIter};

use miette::{NamedSource, SourceSpan};

use crate::{
    errors::ParseError,
    lexer::{Token, TokenKind},
};

#[derive(Debug)]
pub struct Identifier {
    pub name: String,
    pub span: SourceSpan,
}

// span is provenance (for error reporting), not identity: two identifiers
// are equal if their names are, so tests can compare ASTs without
// hardcoding byte offsets.
impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Integer(i64),
    Variable(Identifier),
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

#[derive(Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Return(Expr),
    // todo: this will be changed to parsing some type in the future
    // declarations, but hardcoded to int for now
    Decl(Identifier),
    // "x = expr;"
    Assign(Identifier, Expr),
    // "int x = expr;"
    DeclAssignment(Identifier, Expr),
    // "if (expr) stmt" or "if (expr) stmt else stmt"
    If {
        condition: Expr,
        then: Box<Stmt>,
        else_stmt: Option<Box<Stmt>>,
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
    And,
    Or,
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

    pub(crate) fn parse_program(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();

        // hard code the int main
        if self.peek() == Some(TokenKind::Int) {
            self.expect(TokenKind::Int, "'int'")?;
            self.expect(TokenKind::Main, "'main'")?;
            self.expect(TokenKind::LParen, "'('")?;
            self.expect(TokenKind::RParen, "')'")?;
            self.expect(TokenKind::LBrace, "'{'")?;

            while self.peek() != Some(TokenKind::RBrace) {
                if self.peek().is_none() {
                    return Err(self.expected_error("a statement or '}'"));
                }
                statements.push(self.parse_stmt()?);
            }
            self.expect(TokenKind::RBrace, "'}'")?;
        } else {
            statements.push(self.parse_stmt()?);
        }

        if self.peek().is_some() {
            return Err(self.expected_error("end of input"));
        }

        Ok(statements)
    }

    // stmts are
    //     | "x = expr;" | "int x = expr;" | "int x;" | "return expr;" | "expr;"
    //     | "if (expr) stmt" or "if (expr) stmt else stmt"
    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.peek() == Some(TokenKind::If) {
            self.iter.next();
            // condition
            self.expect(TokenKind::LParen, "'('")?;
            let condition = self.parse_expr()?;
            self.expect(TokenKind::RParen, "')'")?;

            // then, if there is a brace then this would be a block statement
            // but for now this would just be a single statement
            let then = Box::new(self.parse_stmt()?);

            // else is optional
            let else_stmt = if self.peek() == Some(TokenKind::Else) {
                self.iter.next();
                Some(Box::new(self.parse_stmt()?))
            } else {
                None
            };
            return Ok(Stmt::If {
                condition,
                then,
                else_stmt,
            });
        }

        // return expressions
        if self.peek() == Some(TokenKind::Return) {
            self.iter.next();
            let expr = self.parse_expr()?;
            self.expect(TokenKind::Semicolon, "';'")?;
            return Ok(Stmt::Return(expr));
        }

        // declarations like "int x;" and "int x = expr;"
        if self.peek() == Some(TokenKind::Int) {
            self.iter.next();
            if let Some(TokenKind::Identifier(name)) = self.peek() {
                let span = self.iter.peek().unwrap().span;
                self.iter.next();
                let ident = Identifier { name, span };
                if self.peek() == Some(TokenKind::Equal) {
                    self.iter.next();
                    let value = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon, "';'")?;
                    return Ok(Stmt::DeclAssignment(ident, value));
                }
                self.expect(TokenKind::Semicolon, "';'")?;
                return Ok(Stmt::Decl(ident));
            }
            return Err(self.expected_error("an identifier after 'int'"));
        }

        // assignments like "x = expr;"
        if let Some(TokenKind::Identifier(name)) = self.peek() {
            let span = self.iter.peek().unwrap().span;
            self.iter.next();
            let ident = Identifier { name, span };
            if self.peek() == Some(TokenKind::Equal) {
                self.iter.next();
                let value = self.parse_expr()?;
                self.expect(TokenKind::Semicolon, "';'")?;
                return Ok(Stmt::Assign(ident, value));
            }
            self.expect(TokenKind::Semicolon, "';'")?;
            return Ok(Stmt::Expr(Expr::Variable(ident)));
        }

        let expr = self.parse_expr()?;

        // statements should consume their own ;
        self.expect(TokenKind::Semicolon, "';'")?;
        Ok(Stmt::Expr(expr))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_logical_or()
    }

    // logical_or = logical_and (("||") logical_and)*
    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_logical_and()?;
        while let Some(TokenKind::OrOr) = self.peek() {
            self.iter.next();
            let rhs = self.parse_logical_and()?;
            lhs = Expr::Binary {
                op: BinaryOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    // logical_and = equality (("&&") equality)*
    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_equality()?;
        while let Some(TokenKind::AndAnd) = self.peek() {
            self.iter.next();
            let rhs = self.parse_equality()?;
            lhs = Expr::Binary {
                op: BinaryOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
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

    // primary = integer | identifier | "(" expression ")"
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
            Some(TokenKind::Identifier(name)) => {
                let span = self.iter.peek().unwrap().span;
                self.iter.next();
                Ok(Expr::Variable(Identifier { name, span }))
            }
            Some(TokenKind::LParen) => {
                self.iter.next();
                let expression = self.parse_expr()?;
                self.expect(TokenKind::RParen, "')'")?;
                Ok(expression)
            }
            _ => Err(self.expected_error("an integer, identifier or '('")),
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
