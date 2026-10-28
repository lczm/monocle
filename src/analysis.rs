use std::collections::HashSet;

use miette::NamedSource;

use crate::{
    errors::AnalysisError,
    parser::{Expr, Identifier, Stmt},
};

// name resolution check over AST
pub(crate) fn analyze(statements: &[Stmt], source: &str) -> Result<(), AnalysisError> {
    // collect all variable declarations that are used, and check for duplicates
    let mut declared: HashSet<&str> = HashSet::new();
    for statement in statements {
        if let Stmt::Decl(ident) | Stmt::DeclAssignment(ident, _) = statement {
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
        match statement {
            Stmt::Expr(expr) | Stmt::Return(expr) => check_expr(expr, &declared, source)?,
            Stmt::Decl(_) => {}
            Stmt::Assign(identifier, value) | Stmt::DeclAssignment(identifier, value) => {
                check_identifier(identifier, &declared, source)?;
                check_expr(value, &declared, source)?;
            }
        }
    }
    Ok(())
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
