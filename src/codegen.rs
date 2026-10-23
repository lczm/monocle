use crate::{BinaryOp, Expr, Stmt, UnaryOp};

// takes ast and emits RV64IM asm
pub fn codegen(statements: Vec<Stmt>) -> String {
    let mut assembly = String::new();
    emit_line(&mut assembly, ".text");
    emit_line(&mut assembly, ".globl main");
    emit_label(&mut assembly, "main");

    let empty = statements.is_empty();
    let mut has_return = false;
    for statement in statements {
        match statement {
            Stmt::Expr(expr) => emit_expr(&expr, &mut assembly),
            // for now we only support a single return statement
            // and it must be the last statement in the program
            Stmt::Return(expr) => {
                has_return = true;
                emit_expr(&expr, &mut assembly);
                // the expression's value is in a0; skip any statements after it
                emit_line(&mut assembly, "j .Lmain_end");
            }
        }
    }

    if empty {
        emit_line(&mut assembly, "li a0, 0");
    }
    if has_return {
        emit_label(&mut assembly, ".Lmain_end");
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
            // emit the LHS of the binary
            emit_expr(lhs, assembly);
            // expands the stack pointer by 16 bytes
            // we use 16 bytes here even though the LHS is only 8 bytes because
            // RISC-V ABI requires the stack pointer to always be 16-byte aligned
            emit_line(assembly, "addi sp, sp, -16");
            // store the LHS value in a0 t0 the stack (8 bytes)
            emit_line(assembly, "sd a0, 0(sp)");

            // emit the RHS of the binary
            emit_expr(rhs, assembly);
            // load the previously emitted LHS into t0
            emit_line(assembly, "ld t0, 0(sp)");
            // restore the stack pointer by the space we emitted initially
            emit_line(assembly, "addi sp, sp, 16");

            // t0 = LHS (just popped), a0 = RHS (freshly evaluated).
            match op {
                BinaryOp::Add => emit_line(assembly, "add a0, t0, a0"),
                BinaryOp::Subtract => emit_line(assembly, "sub a0, t0, a0"),
                BinaryOp::Multiply => emit_line(assembly, "mul a0, t0, a0"),
                BinaryOp::Divide => emit_line(assembly, "div a0, t0, a0"),
                BinaryOp::Modulo => emit_line(assembly, "rem a0, t0, a0"),

                // slt rd, rs1, rs2 sets rd = rs1 < rs2 (signed).
                // slt rd, rs1, rs2 is rd = rs1 < rs2
                // Less is LHS(t0) < RHS(a0)
                // Greater iS LHS(t0) > RHS(a0) which is a0 < t0
                BinaryOp::Less => emit_line(assembly, "slt a0, t0, a0"),
                BinaryOp::Greater => emit_line(assembly, "slt a0, a0, t0"),
                // <= / >= forms invert the set with a logical-not (xori 1).

                // LessEqual is LHS(t0) <= RHS(a0) which is !(LHS(t0) > RHS(a0)) which is !(a0 < t0)
                // slt a0, a0, t0 sets a0 = a0 < t0, then xori a0, a0, 1 flips to get !(a0 < t0)
                BinaryOp::LessEqual => {
                    emit_line(assembly, "slt a0, a0, t0");
                    emit_line(assembly, "xori a0, a0, 1");
                }
                // GreaterEqual is LHS(t0) >= RHS(a0) which is !(LHS(t0) < RHS(a0)) which is !(t0 < a0)
                BinaryOp::GreaterEqual => {
                    emit_line(assembly, "slt a0, t0, a0");
                    emit_line(assembly, "xori a0, a0, 1");
                }

                // LHS == RHS is !(LHS ^ RHS) which is !(t0 ^ a0)
                BinaryOp::Equal => {
                    emit_line(assembly, "xor a0, t0, a0");
                    emit_line(assembly, "seqz a0, a0");
                }
                // LHS != RHS is (LHS ^ RHS) which is (t0 ^ a0)
                BinaryOp::NotEqual => {
                    emit_line(assembly, "xor a0, t0, a0");
                    emit_line(assembly, "snez a0, a0");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
