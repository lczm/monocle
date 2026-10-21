use super::*;

fn compile(source: &str) -> String {
    let tokens = crate::lex(source).unwrap();
    codegen(crate::parse(tokens, source).unwrap())
}

#[test]
fn empty_program_returns_zero() {
    let expected = r#"	.text
	.globl main
main:
	li a0, 0
	ret
"#;
    assert_eq!(codegen(vec![]), expected);
}

#[test]
fn integer_literals_use_the_full_i64_range() {
    for value in [0, 42, -7, i64::MIN, i64::MAX] {
        let expected = format!(
            r#"	.text
	.globl main
main:
	li a0, {value}
	ret
"#
        );
        assert_eq!(codegen(vec![Expr::Integer(value)]), expected);
    }
}

#[test]
fn unary_plus_and_minus() {
    assert_eq!(compile("+42"), compile("42"));
    let expected = r#"	.text
	.globl main
main:
	li a0, 42
	neg a0, a0
	ret
"#;
    assert_eq!(compile("-42"), expected);
}

#[test]
fn unary_minus_applies_to_compound_operands() {
    let expected = r#"	.text
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
    assert_eq!(compile("-(1 + 2)"), expected);
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
            r#"	.text
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
        assert_eq!(compile(&format!("20 {operator} 3")), expected, "{operator}");
    }
}

#[test]
fn nested_expression_preserves_outer_left_operand() {
    let expected = r#"	.text
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
    assert_eq!(compile("10 - (6 - 2)"), expected);
}

#[test]
fn expressions_run_in_order_and_return_the_last_value() {
    let expected = r#"	.text
	.globl main
main:
	li a0, 42
	li a0, 7
	ret
"#;
    assert_eq!(compile("int main() { 42; 7; }"), expected);
}
