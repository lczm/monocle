use crate::{BinaryOp, Expr, Stmt, UnaryOp};

struct Frame {
    slots: Vec<(String, i32)>,
    size: i32,
}

fn build_frame(statements: &Vec<Stmt>) -> Frame {
    let mut slots = Vec::new();
    let mut next_offset = 0;

    for statement in statements {
        if let Stmt::Decl(name) = statement {
            slots.push((name.clone(), next_offset));
            next_offset += 8;
        }
    }

    Frame {
        slots,
        size: round_up_to_16(next_offset),
    }
}

// RV64 needs to be 16-bytes aligned at all times
fn round_up_to_16(bytes: i32) -> i32 {
    (bytes + 15) / 16 * 16
}

// the stack grows down: reserving bytes moves sp down, releasing moves it up
fn reserve_stack(assembly: &mut String, bytes: i32) {
    emit_line(assembly, &format!("addi sp, sp, -{bytes}"));
}

fn release_stack(assembly: &mut String, bytes: i32) {
    emit_line(assembly, &format!("addi sp, sp, {bytes}"));
}

// todo : return undeclared identifier if cannot find
fn lookup_slot(frame: &Frame, name: &String) -> i32 {
    for (slot_name, offset) in frame.slots.iter() {
        if *slot_name == *name {
            return *offset;
        }
    }
    panic!("undefined variable {name}");
}

// takes ast and emits RV64IM asm
pub fn codegen(statements: Vec<Stmt>) -> String {
    let mut assembly = String::new();
    emit_headers(&mut assembly);

    // todo: when we parse individual functions, we have to build a frame for each function
    // only deal with the main() function for now
    // but for now, this can just be a single frame for the entire program
    let frame = build_frame(&statements);
    let has_frame = frame.size > 0;
    if has_frame {
        reserve_stack(&mut assembly, frame.size);
    }

    let empty = statements.is_empty();
    let mut has_return = false;
    // labels that we need to emit for short-circuit jumps
    // they need to be unique so we increment per use of And / Or
    let mut next_label = 0_u32;
    for statement in statements {
        match statement {
            Stmt::Expr(expr) => emit_expr(&expr, &mut assembly, &mut next_label, &frame),
            // declarations only reserve space at the start when doing the pre-pass
            // it doesnt emit any code
            Stmt::Decl(_) => {}
            // name = expr
            // emit the value lands to a0 then we store it into names slot
            Stmt::Assign(name, value) => {
                emit_expr(&value, &mut assembly, &mut next_label, &frame);
                let offset = lookup_slot(&frame, &name);
                emit_line(&mut assembly, &format!("sd a0, {offset}(sp)"));
            }
            // for now we only support a single return statement
            // and it must be the last statement in the program
            Stmt::Return(expr) => {
                has_return = true;
                emit_expr(&expr, &mut assembly, &mut next_label, &frame);
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

    // give the frame back on both the fallthrough and the return-jump path
    if has_frame {
        release_stack(&mut assembly, frame.size);
    }
    emit_line(&mut assembly, "ret");
    assembly
}

fn emit_headers(assembly: &mut String) {
    emit_line(assembly, ".text");
    emit_line(assembly, ".globl main");
    emit_label(assembly, "main");
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
fn emit_expr(expression: &Expr, assembly: &mut String, next_label: &mut u32, frame: &Frame) {
    match expression {
        Expr::Integer(value) => emit_line(assembly, &format!("li a0, {value}")),
        // load the variable's slot into a0
        Expr::Variable(name) => {
            let offset = lookup_slot(frame, name);
            emit_line(assembly, &format!("ld a0, {offset}(sp)"));
        }
        Expr::Unary { op, operand } => {
            emit_expr(operand, assembly, next_label, frame);
            if *op == UnaryOp::Minus {
                emit_line(assembly, "neg a0, a0");
            }
        }
        Expr::Binary { op, lhs, rhs } => {
            // a && b is 0 when a is 0
            // a || b is 1 when a is nonzero
            // short-circuit for these operators and not evaluate RHS if LHS is enough
            if *op == BinaryOp::And || *op == BinaryOp::Or {
                // emit the LHS of the binary
                emit_expr(lhs, assembly, next_label, frame);
                // unique suffix for this pair of labels, so nested
                // short-circuits do not repeat a label name
                let l = *next_label;
                *next_label += 1;

                // if the LHS alone decides the result, skip the RHS entirely
                if *op == BinaryOp::Or {
                    // if a0 != 0, then we just bump to the true label and skip RHS
                    emit_line(assembly, &format!("bnez a0, .Ltrue{l}"));
                }
                if *op == BinaryOp::And {
                    // if a0 == 0, then we just bump to the false label and skip RHS
                    emit_line(assembly, &format!("beqz a0, .Lfalse{l}"));
                }

                // the LHS did not decide: evaluate the RHS and
                // normalize it to 0 or 1
                emit_expr(rhs, assembly, next_label, frame);
                emit_line(assembly, "snez a0, a0");
                // skip over the constant load of the short-circuit path
                emit_line(assembly, &format!("j .Ldone{l}"));

                // after emitting RHS, we generate the labels for the
                // short circuit paths so the previous paths can be used to skip over
                // the RHS emitted code if needed
                if *op == BinaryOp::Or {
                    emit_label(assembly, &format!(".Ltrue{l}"));
                    emit_line(assembly, "li a0, 1");
                } else {
                    emit_label(assembly, &format!(".Lfalse{l}"));
                    emit_line(assembly, "li a0, 0");
                }
                // emit the label at the end to skip to the end
                emit_label(assembly, &format!(".Ldone{l}"));
                return;
            }

            // emit the LHS of the binary
            emit_expr(lhs, assembly, next_label, frame);
            // expands the stack pointer by 16 bytes
            // we use 16 bytes here even though the LHS is only 8 bytes because
            // RISC-V ABI requires the stack pointer to always be 16-byte aligned
            emit_line(assembly, "addi sp, sp, -16");
            // store the LHS value in a0 t0 the stack (8 bytes)
            emit_line(assembly, "sd a0, 0(sp)");

            // emit the RHS of the binary
            emit_expr(rhs, assembly, next_label, frame);
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
                // handled above so this will never be reached
                BinaryOp::And | BinaryOp::Or => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests;
