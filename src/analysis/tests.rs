use super::*;

fn analyze(source: &str) -> Result<(), AnalysisError> {
    let tokens = crate::lex(source).unwrap();
    let statements = crate::parse(tokens, source).unwrap();
    super::analyze(&statements, source)
}

#[test]
fn accepted_program_passes() {
    let source = "
int main() {
    int x;
    int y = 2;
    x = 1;
    return x + y;
}";
    assert_eq!(analyze(source), Ok(()));
}

#[test]
fn reading_an_undeclared_variable_is_flagged() {
    let source = "
int main() {
    int x;
    return x + z;
}";
    let error = analyze(source).unwrap_err();
    assert!(
        matches!(
            &error,
            AnalysisError::UndefinedVariable { span, name, .. }
                if name == "z" && span.offset() == 40 && span.len() == 1
        ),
        "{error:?}"
    );
}

#[test]
fn writing_to_an_undeclared_variable_is_flagged() {
    let source = "
int main() {
    int x;
    z = 5;
}";
    let error = analyze(source).unwrap_err();
    assert!(
        matches!(
            &error,
            AnalysisError::UndefinedVariable { span, name, .. }
                if name == "z" && span.offset() == 29 && span.len() == 1
        ),
        "{error:?}"
    );
}

#[test]
fn undeclared_variable_in_an_initializer_is_flagged() {
    let source = "
int main() {
    int x = z;
}";
    let error = analyze(source).unwrap_err();
    assert!(
        matches!(
            &error,
            AnalysisError::UndefinedVariable { span, name, .. }
                if name == "z" && span.offset() == 26 && span.len() == 1
        ),
        "{error:?}"
    );
}

#[test]
fn duplicate_declaration_is_flagged() {
    let source = "
int main() {
    int x;
    int x;
}";
    let error = analyze(source).unwrap_err();
    assert!(
        matches!(
            &error,
            AnalysisError::DuplicateDeclaration { span, name, .. }
                if name == "x" && span.offset() == 33 && span.len() == 1
        ),
        "{error:?}"
    );
}

#[test]
fn undeclared_variable_in_an_if_condition_is_flagged() {
    let source = "
int main() {
    if (z) x = 1;
}";
    let error = analyze(source).unwrap_err();
    assert!(
        matches!(
            &error,
            AnalysisError::UndefinedVariable { span, name, .. }
                if name == "z" && span.offset() == 22 && span.len() == 1
        ),
        "{error:?}"
    );
}

#[test]
fn undeclared_variable_in_an_if_body_is_flagged() {
    let source = "
int main() {
    int x;
    if (x) z = 1; else x = 2;
}";
    let error = analyze(source).unwrap_err();
    assert!(
        matches!(
            &error,
            AnalysisError::UndefinedVariable { span, name, .. }
                if name == "z" && span.offset() == 36 && span.len() == 1
        ),
        "{error:?}"
    );
}

#[test]
fn if_bodies_can_use_declared_variables() {
    let source = "
int main() {
    int x;
    if (x) x = 1; else x = 2;
}";
    assert_eq!(analyze(source), Ok(()));
}
