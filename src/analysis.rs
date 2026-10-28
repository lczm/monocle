use std::collections::HashSet;

use miette::NamedSource;

use crate::{
    errors::AnalysisError,
    parser::{Expr, Identifier, Statement},
};

// name resolution check over AST
pub(crate) fn analyze(statements: &[Statement], source: &str) -> Result<(), AnalysisError> {
    // collect all variable declarations that are used, and check for duplicates
    let mut declared: HashSet<&str> = HashSet::new();
    for statement in statements {
        if let Statement::Decl(ident) | Statement::DeclAssignment(ident, _) = statement {
            if !declared.insert(ident.name.as_str()) {
                return Err(AnalysisError::DuplicateDeclaration {
                    src: NamedSource::new("input", source.to_string()),
                    span: ident.span,
                    name: ident.name.clone(),
                });
            }
        }
    }

    // run through all statements and check expr / identifier
    for statement in statements {
        check_stmt(statement, &declared, source)?;
    }
    Ok(())
}

// statements nest (if bodies), so checks recurse through the tree
fn check_stmt(
    statement: &Statement,
    declared: &HashSet<&str>,
    source: &str,
) -> Result<(), AnalysisError> {
    match statement {
        Statement::Expr(expr) | Statement::Return(expr) => check_expr(expr, declared, source),
        Statement::Decl(_) => Ok(()),
        Statement::Assign(identifier, value) | Statement::DeclAssignment(identifier, value) => {
            check_identifier(identifier, declared, source)?;
            check_expr(value, declared, source)
        }
        Statement::If {
            condition,
            then,
            else_stmt,
        } => {
            check_expr(condition, declared, source)?;
            check_stmt(then, declared, source)?;
            // only if there is an else statement then check
            if let Some(else_stmt) = else_stmt {
                check_stmt(else_stmt, declared, source)?;
            }
            Ok(())
        }
    }
}

fn check_identifier(
    identifier: &Identifier,
    declared: &HashSet<&str>,
    source: &str,
) -> Result<(), AnalysisError> {
    if declared.contains(identifier.name.as_str()) {
        return Ok(());
    }
    Err(AnalysisError::UndefinedVariable {
        src: NamedSource::new("input", source.to_string()),
        span: identifier.span,
        name: identifier.name.clone(),
    })
}

fn check_expr(expr: &Expr, declared: &HashSet<&str>, source: &str) -> Result<(), AnalysisError> {
    match expr {
        Expr::Unary { op: _, operand } => check_expr(operand, declared, source),
        Expr::Binary { op: _, lhs, rhs } => {
            check_expr(lhs, declared, source)?;
            check_expr(rhs, declared, source)
        }
        Expr::Variable(ident) => check_identifier(ident, declared, source),
        Expr::Integer(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests;
