use crate::{BinaryOp, Expr, UnaryOp};

// takes ast and emits RV64IM asm
pub fn codegen(expressions: Vec<Expr>) -> String {
    let mut assembly = String::new();
    emit_line(&mut assembly, ".text");
    emit_line(&mut assembly, ".globl main");
    emit_label(&mut assembly, "main");

    if expressions.is_empty() {
        emit_line(&mut assembly, "li a0, 0");
    }
    for expression in &expressions {
        emit_expr(expression, &mut assembly);
    }

    emit_line(&mut assembly, "ret");
    assembly
}

fn emit_line(assembly: &mut String, line: &str) {
    assembly.push('\t');
    assembly.push_str(line);
    assembly.push('\n');
}

fn emit_label(assembly: &mut String, label: &str) {
    assembly.push_str(label);
    assembly.push_str(":\n");
}

// Every expression leaves its result in a0 and restores sp.
fn emit_expr(expression: &Expr, assembly: &mut String) {
    match expression {
        Expr::Integer(value) => emit_line(assembly, &format!("li a0, {value}")),
        Expr::Unary { op, operand } => {
            emit_expr(operand, assembly);
            if *op == UnaryOp::Minus {
                emit_line(assembly, "neg a0, a0");
            }
        }
        Expr::Binary { op, lhs, rhs } => {
            emit_expr(lhs, assembly);
            // Preserve the left operand while evaluating the right operand.
            // A 16-byte slot keeps sp aligned to the RISC-V ABI.
            emit_line(assembly, "addi sp, sp, -16");
            emit_line(assembly, "sd a0, 0(sp)");
            emit_expr(rhs, assembly);
            emit_line(assembly, "ld t0, 0(sp)");
            emit_line(assembly, "addi sp, sp, 16");

            let instruction = match op {
                BinaryOp::Add => "add",
                BinaryOp::Subtract => "sub",
                BinaryOp::Multiply => "mul",
                BinaryOp::Divide => "div",
                BinaryOp::Modulo => "rem",
            };
            emit_line(assembly, &format!("{instruction} a0, t0, a0"));
        }
    }
}

#[cfg(test)]
mod tests;
