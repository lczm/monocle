use std::collections::HashMap;

use crate::{BinaryOp, Block, Expr, Statement, UnaryOp, errors::CodegenError};

struct Frame {
    slots: Vec<(String, i32)>,
    size: i32,
}

fn build_frame(body: &Block) -> Frame {
    let mut slots = Vec::new();
    let mut next_offset = 0;
    collect_slots(&body.statements, &mut slots, &mut next_offset);

    Frame {
        slots,
        size: round_up_to_16(next_offset),
    }
}

fn collect_slots(statements: &[Statement], slots: &mut Vec<(String, i32)>, next_offset: &mut i32) {
    for statement in statements {
        match statement {
            Statement::Decl(identifier) | Statement::DeclAssignment(identifier, _) => {
                slots.push((identifier.name.clone(), *next_offset));
                *next_offset += 8;
            }
            Statement::Block(Block { statements }) => collect_slots(statements, slots, next_offset),
            // nothing else can declare new names
            _ => {}
        }
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

fn declare_slot(
    frame: &Frame,
    slot_index: &mut usize,
    scopes: &mut [HashMap<String, i32>],
) -> Result<i32, CodegenError> {
    // the pre-pass slots are in DFS order, so consume it on a declaration statement
    let (name, offset) = &frame.slots[*slot_index];
    *slot_index += 1;
    // insert the name and its offset into the current scope, which is the last one in the stack
    scopes
        .last_mut()
        .expect("popped global scope, should never happen")
        .insert(name.clone(), *offset);
    Ok(*offset)
}

// an identifier being used must be declared in some scope; we check the
// scopes inside out, like analysis
fn lookup_slot(scopes: &[HashMap<String, i32>], name: &str) -> Result<i32, CodegenError> {
    for scope in scopes.iter().rev() {
        if let Some(&offset) = scope.get(name) {
            return Ok(offset);
        }
    }
    Err(CodegenError {
        name: name.to_owned(),
    })
}

// takes ast and emits RV64IM asm
pub fn codegen(statements: Vec<Statement>) -> Result<String, CodegenError> {
    let mut assembly = String::new();
    emit_headers(&mut assembly);

    let empty = statements.is_empty();
    // put the whole program in 1 block for now, when we have functions
    // then it emits one block per function
    let body = Block { statements };

    // todo: when we parse individual functions, we have to build a frame for each function
    // but for now, this can just be a single frame for the entire program
    let frame = build_frame(&body);
    let has_frame = frame.size > 0;
    if has_frame {
        reserve_stack(&mut assembly, frame.size);
        emit_frame_pointer(&mut assembly);
    }

    let mut has_return = false;
    // labels that we need to emit for short-circuit jumps
    // they need to be unique so we increment per use of And / Or / if
    let mut next_label = 0_u32;

    // scope stack that pushes and pops, add a global scope
    let mut scopes: Vec<HashMap<String, i32>> = Vec::new();
    scopes.push(HashMap::new());

    // the walk consumes the pre-pass slots in the same DFS order
    let mut slot_index = 0;

    // run through the whole body and emit code for each statement in order
    emit_statement(
        &Statement::Block(body),
        &mut assembly,
        &mut next_label,
        &frame,
        &mut has_return,
        &mut scopes,
        &mut slot_index,
    )?;

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
    Ok(assembly)
}

fn emit_frame_pointer(assembly: &mut String) {
    // todo: s0 is callee-saved, so once functions can call each other
    // it must be saved and restored in the prologue and epilogue
    emit_line(assembly, "mv s0, sp");
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

fn emit_statement(
    statement: &Statement,
    assembly: &mut String,
    next_label: &mut u32,
    frame: &Frame,
    has_return: &mut bool,
    scopes: &mut Vec<HashMap<String, i32>>,
    slot_index: &mut usize,
) -> Result<(), CodegenError> {
    match statement {
        Statement::Expr(expr) => emit_expr(expr, assembly, next_label, scopes)?,
        // declarations reserve their slot in the pre-pass; the walk only
        // registers the name in its scope
        Statement::Decl(_) => {
            declare_slot(frame, slot_index, scopes)?;
        }
        // emit the value lands to a0 then we store it into names slot
        Statement::Assign(identifier, value) => {
            emit_expr(value, assembly, next_label, scopes)?;
            let offset = lookup_slot(scopes, &identifier.name)?;
            emit_line(assembly, &format!("sd a0, {offset}(s0)"));
        }
        // declaration first, so the initializer can reference the name
        Statement::DeclAssignment(_, value) => {
            let offset = declare_slot(frame, slot_index, scopes)?;
            emit_expr(value, assembly, next_label, scopes)?;
            emit_line(assembly, &format!("sd a0, {offset}(s0)"));
        }
        Statement::Return(expr) => {
            *has_return = true;
            emit_expr(expr, assembly, next_label, scopes)?;
            // the expression's value is in a0; skip any statements after it
            emit_line(assembly, "j .Lmain_end");
        }
        Statement::If {
            condition,
            then,
            else_stmt,
        } => {
            // generate the condition
            emit_expr(condition, assembly, next_label, scopes)?;
            // unique suffix for this if, so nested ifs and short-circuits in
            // the condition do not collide with its labels
            let l = *next_label;
            *next_label += 1;

            if let Some(else_stmt) = else_stmt {
                // condition is false, we jump to the else branch, skipping then statement
                emit_line(assembly, &format!("beqz a0, .Lelse{l}"));
                // emit the then branch which gets ran if condition is true
                emit_statement(
                    then, assembly, next_label, frame, has_return, scopes, slot_index,
                )?;
                // jump to the end of the if statement, skipping the else branch
                emit_line(assembly, &format!("j .Lend{l}"));
                // emit the else branch with its label
                emit_label(assembly, &format!(".Lelse{l}"));
                emit_statement(
                    else_stmt, assembly, next_label, frame, has_return, scopes, slot_index,
                )?;
                // skips to the end
                emit_label(assembly, &format!(".Lend{l}"));
            } else {
                // falsy condition jumps straight to the end
                emit_line(assembly, &format!("beqz a0, .Lend{l}"));
                emit_statement(
                    then, assembly, next_label, frame, has_return, scopes, slot_index,
                )?;
                emit_label(assembly, &format!(".Lend{l}"));
            }
        }
        Statement::While { condition, body } => {
            // create a label so can jump back to the top of the loop
            let l = *next_label;
            *next_label += 1;
            emit_label(assembly, &format!(".Lcond{l}"));

            // the condition evaluation
            emit_expr(condition, assembly, next_label, scopes)?;
            // condition is false, go to the end and skip the body
            emit_line(assembly, &format!("beqz a0, .Lend{l}"));

            // the body of the loop
            emit_statement(
                body, assembly, next_label, frame, has_return, scopes, slot_index,
            )?;
            // loop back to the top and reeval the condition
            emit_line(assembly, &format!("j .Lcond{l}"));
            emit_label(assembly, &format!(".Lend{l}"));
        }
        Statement::Block(Block { statements }) => {
            // a block is a new scope: its names must disappear when it ends
            scopes.push(HashMap::new());
            // the pop must run even when a statement inside fails
            let result = statements.iter().try_for_each(|stmt| {
                emit_statement(
                    stmt, assembly, next_label, frame, has_return, scopes, slot_index,
                )
            });
            scopes.pop();
            result?;
        }
    }
    Ok(())
}

// Every expression leaves its result in a0 and restores sp.
fn emit_expr(
    expression: &Expr,
    assembly: &mut String,
    next_label: &mut u32,
    scopes: &[HashMap<String, i32>],
) -> Result<(), CodegenError> {
    match expression {
        Expr::Integer(value) => {
            emit_line(assembly, &format!("li a0, {value}"));
            Ok(())
        }
        // load the variable's slot into a0
        Expr::Variable(identifier) => {
            let offset = lookup_slot(scopes, &identifier.name)?;
            emit_line(assembly, &format!("ld a0, {offset}(s0)"));
            Ok(())
        }
        Expr::Unary { op, operand } => {
            emit_expr(operand, assembly, next_label, scopes)?;
            if *op == UnaryOp::Minus {
                emit_line(assembly, "neg a0, a0");
            }
            Ok(())
        }
        Expr::Binary { op, lhs, rhs } => {
            // a && b is 0 when a is 0
            // a || b is 1 when a is nonzero
            // short-circuit for these operators and not evaluate RHS if LHS is enough
            if *op == BinaryOp::And || *op == BinaryOp::Or {
                // emit the LHS of the binary
                emit_expr(lhs, assembly, next_label, scopes)?;
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
                emit_expr(rhs, assembly, next_label, scopes)?;
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
                return Ok(());
            }

            // emit the LHS of the binary
            emit_expr(lhs, assembly, next_label, scopes)?;
            // expands the stack pointer by 16 bytes
            // we use 16 bytes here even though the LHS is only 8 bytes because
            // RISC-V ABI requires the stack pointer to always be 16-byte aligned
            emit_line(assembly, "addi sp, sp, -16");
            // store the LHS value in a0 t0 the stack (8 bytes)
            emit_line(assembly, "sd a0, 0(sp)");

            // emit the RHS of the binary
            emit_expr(rhs, assembly, next_label, scopes)?;
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
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
