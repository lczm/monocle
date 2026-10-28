use super::*;

fn compile(source: &str) -> String {
    let tokens = crate::lex(source).unwrap();
    codegen(crate::parse(tokens, source).unwrap()).unwrap()
}

fn asm_eq(actual: impl AsRef<str>, expected: impl AsRef<str>) {
    let expected = expected.as_ref();
    assert_eq!(
        actual.as_ref(),
        expected.strip_prefix('\n').unwrap_or(expected)
    );
}

#[test]
fn empty_program_returns_zero() {
    let expected = r#"
	.text
	.globl main
main:
	li a0, 0
	ret
"#;
    asm_eq(codegen(vec![]).unwrap(), expected);
}

#[test]
fn integer_literals_use_the_full_i64_range() {
    for value in [0, 42, -7, i64::MIN, i64::MAX] {
        let expected = format!(
            r#"
	.text
	.globl main
main:
	li a0, {value}
	ret
"#
        );
        asm_eq(
            codegen(vec![Statement::Expr(Expr::Integer(value))]).unwrap(),
            expected,
        );
    }
}

#[test]
fn unary_plus_and_minus() {
    assert_eq!(compile("+42;"), compile("42;"));
    let expected = r#"
	.text
	.globl main
main:
	li a0, 42
	neg a0, a0
	ret
"#;
    asm_eq(compile("-42;"), expected);
}

#[test]
fn unary_minus_applies_to_compound_operands() {
    let expected = r#"
	.text
	.globl main
main:
	li a0, 1
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 2
	ld t0, 0(sp)
	addi sp, sp, 16
	add a0, t0, a0
	neg a0, a0
	ret
"#;
    asm_eq(compile("-(1 + 2);"), expected);
}

#[test]
fn binary_operators_keep_left_and_right_operands_in_order() {
    for (operator, instruction) in [
        ("+", "add"),
        ("-", "sub"),
        ("*", "mul"),
        ("/", "div"),
        ("%", "rem"),
    ] {
        let expected = format!(
            r#"
	.text
	.globl main
main:
	li a0, 20
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 3
	ld t0, 0(sp)
	addi sp, sp, 16
	{instruction} a0, t0, a0
	ret
"#
        );
        assert_eq!(
            compile(&format!("20 {operator} 3;")),
            expected.strip_prefix('\n').unwrap_or(expected.as_str()),
            "{operator}"
        );
    }
}

#[test]
fn comparison_operators_emit_set_instructions() {
    for (operator, instructions) in [
        ("<", "slt a0, t0, a0"),
        (">", "slt a0, a0, t0"),
        ("<=", "slt a0, a0, t0\n\txori a0, a0, 1"),
        (">=", "slt a0, t0, a0\n\txori a0, a0, 1"),
        ("==", "xor a0, t0, a0\n\tseqz a0, a0"),
        ("!=", "xor a0, t0, a0\n\tsnez a0, a0"),
    ] {
        let expected = format!(
            r#"
	.text
	.globl main
main:
	li a0, 20
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 3
	ld t0, 0(sp)
	addi sp, sp, 16
	{instructions}
	ret
"#
        );
        assert_eq!(
            compile(&format!("20 {operator} 3;")),
            expected.strip_prefix('\n').unwrap_or(expected.as_str()),
            "{operator}"
        );
    }
}

#[test]
fn nested_expression_preserves_outer_left_operand() {
    let expected = r#"
	.text
	.globl main
main:
	li a0, 10
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 6
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 2
	ld t0, 0(sp)
	addi sp, sp, 16
	sub a0, t0, a0
	ld t0, 0(sp)
	addi sp, sp, 16
	sub a0, t0, a0
	ret
"#;
    asm_eq(compile("10 - (6 - 2);"), expected);
}

#[test]
fn expressions_run_in_order_and_return_the_last_value() {
    let source = "
int main() {
    42;
    7;
}";

    let expected = r#"
	.text
	.globl main
main:
	li a0, 42
	li a0, 7
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn return_jumps_to_the_end_ignoring_other_statements() {
    let source = "
int main() {
    return 42;
    7;
}";

    let expected = r#"
	.text
	.globl main
main:
	li a0, 42
	j .Lmain_end
	li a0, 7
.Lmain_end:
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn return_after_other_statements_returns_the_computed_value() {
    let source = "
int main() {
    7;
    return 2 * 3;
}";

    let expected = r#"
	.text
	.globl main
main:
	li a0, 7
	li a0, 2
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 3
	ld t0, 0(sp)
	addi sp, sp, 16
	mul a0, t0, a0
	j .Lmain_end
.Lmain_end:
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn logical_and_emits_short_circuit_jump() {
    let expected = r#"
	.text
	.globl main
main:
	li a0, 1
	beqz a0, .Lfalse0
	li a0, 2
	snez a0, a0
	j .Ldone0
.Lfalse0:
	li a0, 0
.Ldone0:
	ret
"#;
    asm_eq(compile("1 && 2;"), expected);
}

#[test]
fn logical_or_emits_short_circuit_jump() {
    let expected = r#"
	.text
	.globl main
main:
	li a0, 1
	bnez a0, .Ltrue0
	li a0, 0
	snez a0, a0
	j .Ldone0
.Ltrue0:
	li a0, 1
.Ldone0:
	ret
"#;
    asm_eq(compile("1 || 0;"), expected);
}

#[test]
fn short_circuit_places_the_branch_before_the_rhs() {
    let expected = r#"
	.text
	.globl main
main:
	li a0, 0
	beqz a0, .Lfalse0
	li a0, 1
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 2
	ld t0, 0(sp)
	addi sp, sp, 16
	add a0, t0, a0
	snez a0, a0
	j .Ldone0
.Lfalse0:
	li a0, 0
.Ldone0:
	ret
"#;
    asm_eq(compile("0 && (1 + 2);"), expected);
}

#[test]
fn chained_logical_operators_number_their_labels() {
    let expected = r#"
	.text
	.globl main
main:
	li a0, 1
	beqz a0, .Lfalse0
	li a0, 0
	snez a0, a0
	j .Ldone0
.Lfalse0:
	li a0, 0
.Ldone0:
	bnez a0, .Ltrue1
	li a0, 1
	snez a0, a0
	j .Ldone1
.Ltrue1:
	li a0, 1
.Ldone1:
	ret
"#;
    asm_eq(compile("1 && 0 || 1;"), expected);
}

#[test]
fn declarations_allocate_a_frame() {
    let source = "
int main() {
    int x;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn frame_size_rounds_up_to_16() {
    // two 8-byte slots fit in 16 bytes
    let source = "
int main() {
    int x;
    int y;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);

    // three 8-byte slots are 24 bytes, which rounds up to 32 (a 16-multiple)
    let source = "
int main() {
    int x;
    int y;
    int z;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -32
	addi sp, sp, 32
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn declarations_coexist_with_statements_and_return() {
    let source = "
int main() {
    int x;
    1;
    return 2;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	li a0, 1
	li a0, 2
	j .Lmain_end
.Lmain_end:
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn assignment_stores_into_the_variable_slot() {
    let source = "
int main() {
    int x;
    x = 2 + 3;
    return x;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	li a0, 2
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 3
	ld t0, 0(sp)
	addi sp, sp, 16
	add a0, t0, a0
	sd a0, 0(sp)
	ld a0, 0(sp)
	j .Lmain_end
.Lmain_end:
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn declaration_with_initializer_stores_into_the_variable_slot() {
    let source = "
int main() {
    int x = 2 + 3;
    return x;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	li a0, 2
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 3
	ld t0, 0(sp)
	addi sp, sp, 16
	add a0, t0, a0
	sd a0, 0(sp)
	ld a0, 0(sp)
	j .Lmain_end
.Lmain_end:
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn declaration_with_initializer_emits_the_same_code_as_separate_assignment() {
    // "int x = e;" should behave exactly like "int x; x = e;"
    let with_initializer = "
int main() {
    int x = 2 + 3;
    return x;
}";
    let separate_assignment = "
int main() {
    int x;
    x = 2 + 3;
    return x;
}";
    assert_eq!(compile(with_initializer), compile(separate_assignment));
}

#[test]
fn multiple_variables_get_distinct_slots() {
    let source = "
int main() {
    int x;
    int y;
    x = 1;
    y = 2;
    return x + y;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	li a0, 1
	sd a0, 0(sp)
	li a0, 2
	sd a0, 8(sp)
	ld a0, 0(sp)
	addi sp, sp, -16
	sd a0, 0(sp)
	ld a0, 8(sp)
	ld t0, 0(sp)
	addi sp, sp, 16
	add a0, t0, a0
	j .Lmain_end
.Lmain_end:
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn reading_or_writing_an_undeclared_variable_is_an_error() {
    // same error for both reading and writing
    let expected_error = crate::errors::CodegenError {
        name: "z".to_string(),
    };

    // reading `z` inside an expression
    let source = "
int main() {
    int x;
    int y;
    x = 1;
    y = 2;
    return x + z;
}";
    let tokens = crate::lex(source).unwrap();
    let statements = crate::parse(tokens, source).unwrap();
    assert_eq!(codegen(statements).unwrap_err(), expected_error);

    // writing to `z` with an assignment
    let source = "
int main() {
    int x;
    int y;
    x = 1;
    y = 2;
    z = 2;
}";
    let tokens = crate::lex(source).unwrap();
    let statements = crate::parse(tokens, source).unwrap();
    assert_eq!(codegen(statements).unwrap_err(), expected_error);
}

#[test]
fn if_else_takes_the_taken_branch() {
    let source = "
int main() {
    int x;
    if (0)
		x = 40;
	else
		x = 2;
    return x;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	li a0, 0
	beqz a0, .Lelse0
	li a0, 40
	sd a0, 0(sp)
	j .Lend0
.Lelse0:
	li a0, 2
	sd a0, 0(sp)
.Lend0:
	ld a0, 0(sp)
	j .Lmain_end
.Lmain_end:
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn if_without_else_skips_the_body_to_the_end() {
    let source = "
int main() {
    int x = 1;
    if (x == 0) x = 2;
    return x;
}";
    let expected = r#"
	.text
	.globl main
main:
	addi sp, sp, -16
	li a0, 1
	sd a0, 0(sp)
	ld a0, 0(sp)
	addi sp, sp, -16
	sd a0, 0(sp)
	li a0, 0
	ld t0, 0(sp)
	addi sp, sp, 16
	xor a0, t0, a0
	seqz a0, a0
	beqz a0, .Lend0
	li a0, 2
	sd a0, 0(sp)
.Lend0:
	ld a0, 0(sp)
	j .Lmain_end
.Lmain_end:
	addi sp, sp, 16
	ret
"#;
    asm_eq(compile(source), expected);
}

#[test]
fn returning_inside_an_if_skips_the_rest_of_the_function() {
    let source = "
int main() {
    if (1) return 40;
    return 0;
}";
    let expected = r#"
	.text
	.globl main
main:
	li a0, 1
	beqz a0, .Lend0
	li a0, 40
	j .Lmain_end
.Lend0:
	li a0, 0
	j .Lmain_end
.Lmain_end:
	ret
"#;
    asm_eq(compile(source), expected);
}
