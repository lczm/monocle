use std::collections::HashSet;

use miette::NamedSource;

use crate::{
    errors::AnalysisError,
    parser::{Block, Expr, Identifier, Statement},
};

// name resolution check over AST
pub(crate) fn analyze(statements: &[Statement], source: &str) -> Result<(), AnalysisError> {
    // pass scope stack through
    // statements can push and pop scopes along the way like blocks
    // the last scope is the current scope
    let mut scopes: Vec<HashSet<String>> = vec![HashSet::new()];
    for statement in statements {
        check_stmt(statement, &mut scopes, source)?;
    }
    Ok(())
}

// the scope stack owns its names, so the walk holds no borrow from the AST
fn check_stmt(
    statement: &Statement,
    scopes: &mut Vec<HashSet<String>>,
    source: &str,
) -> Result<(), AnalysisError> {
    match statement {
        Statement::Expr(expr) | Statement::Return(expr) => check_expr(expr, scopes, source),
        Statement::Decl(identifier) => declare(identifier, scopes, source),
        Statement::DeclAssignment(identifier, value) => {
            // declare then check the expression that is initialized
            declare(identifier, scopes, source)?;
            check_expr(value, scopes, source)
        }
        Statement::Assign(identifier, value) => {
            check_identifier(identifier, scopes, source)?;
            check_expr(value, scopes, source)
        }
        Statement::If {
            condition,
            then,
            else_stmt,
        } => {
            check_expr(condition, scopes, source)?;
            check_stmt(then, scopes, source)?;
            // only if there is an else statement then check
            if let Some(else_stmt) = else_stmt {
                check_stmt(else_stmt, scopes, source)?;
            }
            Ok(())
        }
        Statement::Block(Block { statements }) => {
            // a block is a new scope: its names must disappear when it ends
            scopes.push(HashSet::new());
            // the pop must run even when a statement inside fails
            let result = statements
                .iter()
                .try_for_each(|stmt| check_stmt(stmt, scopes, source));
            scopes.pop();
            result
        }
    }
}

// an identifier being used must be declared in some scope
// we check the scope inside out
fn check_identifier(
    identifier: &Identifier,
    scopes: &[HashSet<String>],
    source: &str,
) -> Result<(), AnalysisError> {
    for scope in scopes.iter().rev() {
        if scope.contains(identifier.name.as_str()) {
            return Ok(());
        }
    }
    Err(AnalysisError::UndefinedVariable {
        src: NamedSource::new("input", source.to_string()),
        span: identifier.span,
        name: identifier.name.clone(),
    })
}

fn check_expr(expr: &Expr, scopes: &[HashSet<String>], source: &str) -> Result<(), AnalysisError> {
    match expr {
        Expr::Unary { op: _, operand } => check_expr(operand, scopes, source),
        Expr::Binary { op: _, lhs, rhs } => {
            check_expr(lhs, scopes, source)?;
            check_expr(rhs, scopes, source)
        }
        Expr::Variable(ident) => check_identifier(ident, scopes, source),
        Expr::Integer(_) => Ok(()),
    }
}

// possible optimization is to borrow the names &'a str instead of cloning each
fn declare(
    identifier: &Identifier,
    scopes: &mut [HashSet<String>],
    source: &str,
) -> Result<(), AnalysisError> {
    // only happens if we pop the global scope which never happens
    let innermost = scopes
        .last_mut()
        .expect("popped global scope, should never happen");
    if !innermost.insert(identifier.name.clone()) {
        return Err(AnalysisError::DuplicateDeclaration {
            src: NamedSource::new("input", source.to_string()),
            span: identifier.span,
            name: identifier.name.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
