use ethereum_types::U256;
use Item::{Push, StandardOp};
use PushTarget::Literal;

use crate::cpu::kernel::ast::Item::LocalLabelDeclaration;
use crate::cpu::kernel::ast::PushTarget::Label;
use crate::cpu::kernel::ast::{Item, PushTarget};
use crate::cpu::kernel::utils::replace_windows;

pub(crate) fn optimize_asm(code: &mut Vec<Item>) {
    constant_propagation(code);

    // Remove no-op jumps: [PUSH label, JUMP, label:] -> [label:]
    replace_windows(code, |window| {
        if let [Push(Label(l1)), StandardOp(jump), LocalLabelDeclaration(l2)] = window
            && l1 == l2
            && &jump == "JUMP"
        {
            Some(vec![LocalLabelDeclaration(l2)])
        } else {
            None
        }
    });

    // Remove swaps: [PUSH x, PUSH y, SWAP1] -> [PUSH y, PUSH x]
    replace_windows(code, |window| {
        if let [Push(Literal(x)), Push(Literal(y)), StandardOp(swap1)] = window
                && &swap1 == "SWAP1" {
            Some(vec![Push(Literal(y)), Push(Literal(x))])
        } else {
            None
        }
    });
}

fn constant_propagation(code: &mut Vec<Item>) {
    // Constant propagation for unary ops: [PUSH x, UNARYOP] -> [PUSH UNARYOP(x)]
    replace_windows(code, |window| {
        if let [Push(Literal(x)), StandardOp(op)] = window {
            match op.as_str() {
                "ISZERO" => Some(vec![Push(Literal(if x.is_zero() {
                    U256::one()
                } else {
                    U256::zero()
                }))]),
                "NOT" => Some(vec![Push(Literal(!x))]),
                _ => None,
            }
        } else {
            None
        }
    });

    // Constant propagation for binary ops: [PUSH x, PUSH y, BINOP] -> [PUSH BINOP(x, y)]
    replace_windows(code, |window| {
        if let [Push(Literal(x)), Push(Literal(y)), StandardOp(op)] = window {
            match op.as_str() {
                "ADD" => Some(vec![Push(Literal(x + y))]),
                "SUB" => Some(vec![Push(Literal(x - y))]),
                "MUL" => Some(vec![Push(Literal(x * y))]),
                "DIV" => Some(vec![Push(Literal(x / y))]),
                _ => None,
            }
        } else {
            None
        }
    });
}

#[cfg(test)]
mod tests {}
