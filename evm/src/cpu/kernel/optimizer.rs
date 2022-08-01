use ethereum_types::U256;
use Item::{Push, StandardOp};
use PushTarget::Literal;

use crate::cpu::kernel::ast::Item::{GlobalLabelDeclaration, LocalLabelDeclaration};
use crate::cpu::kernel::ast::PushTarget::Label;
use crate::cpu::kernel::ast::{Item, PushTarget};
use crate::cpu::kernel::utils::replace_windows;

pub(crate) fn optimize_asm(code: &mut Vec<Item>) {
    // Run the optimizer until nothing changes.
    loop {
        let old_code = code.clone();
        optimize_asm_once(code);
        if code == &old_code {
            break;
        }
    }
}

/// A single optimization pass.
fn optimize_asm_once(code: &mut Vec<Item>) {
    constant_propagation(code);
    no_op_jumps(code);
    remove_swaps(code);
    remove_ignored_values(code);
}

/// Constant propagation.
fn constant_propagation(code: &mut Vec<Item>) {
    // Constant propagation for unary ops: `[PUSH x, UNARYOP] -> [PUSH UNARYOP(x)]`
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

    // Constant propagation for binary ops: `[PUSH y, PUSH x, BINOP] -> [PUSH BINOP(x, y)]`
    replace_windows(code, |window| {
        if let [Push(Literal(y)), Push(Literal(x)), StandardOp(op)] = window {
            match op.as_str() {
                "ADD" => Some(vec![Push(Literal(x.overflowing_add(y).0))]),
                "SUB" => Some(vec![Push(Literal(x.overflowing_sub(y).0))]),
                "MUL" => Some(vec![Push(Literal(x.overflowing_mul(y).0))]),
                "DIV" => Some(vec![Push(Literal(
                    x.checked_div(y).unwrap_or(U256::zero()),
                ))]),
                _ => None,
            }
        } else {
            None
        }
    });
}

/// Remove no-op jumps: `[PUSH label, JUMP, label:] -> [label:]`.
fn no_op_jumps(code: &mut Vec<Item>) {
    replace_windows(code, |window| {
        if let [Push(Label(l)), StandardOp(jump), decl] = window
            && &jump == "JUMP"
            && (decl == LocalLabelDeclaration(l.clone()) || decl == GlobalLabelDeclaration(l.clone()))
        {
            Some(vec![LocalLabelDeclaration(l)])
        } else {
            None
        }
    });
}

/// Remove swaps: `[PUSH x, PUSH y, SWAP1] -> [PUSH y, PUSH x]`.
fn remove_swaps(code: &mut Vec<Item>) {
    replace_windows(code, |window| {
        if let [Push(x), Push(y), StandardOp(swap1)] = window
            && &swap1 == "SWAP1" {
            Some(vec![Push(y), Push(x)])
        } else {
            None
        }
    });
}

/// Remove push-pop type patterns, such as: `[DUP1, POP]`.
fn remove_ignored_values(code: &mut Vec<Item>) {
    replace_windows(code, |[a, b]| {
        if let StandardOp(pop) = b && &pop == "POP" {
            match a {
                Push(_) => Some(vec![]),
                StandardOp(dup) if dup.starts_with("DUP") => Some(vec![]),
                _ => None,
            }
        } else {
            None
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_propagation_iszero() {
        let mut code = vec![Push(Literal(3.into())), StandardOp("ISZERO".into())];
        constant_propagation(&mut code);
        assert_eq!(code, vec![Push(Literal(0.into()))]);
    }

    #[test]
    fn test_constant_propagation_add_overflowing() {
        let mut code = vec![
            Push(Literal(U256::max_value())),
            Push(Literal(U256::max_value())),
            StandardOp("ADD".into()),
        ];
        constant_propagation(&mut code);
        assert_eq!(code, vec![Push(Literal(U256::max_value() - 1))]);
    }

    #[test]
    fn test_constant_propagation_sub_underflowing() {
        let mut code = vec![
            Push(Literal(U256::one())),
            Push(Literal(U256::zero())),
            StandardOp("SUB".into()),
        ];
        constant_propagation(&mut code);
        assert_eq!(code, vec![Push(Literal(U256::max_value()))]);
    }

    #[test]
    fn test_constant_propagation_mul() {
        let mut code = vec![
            Push(Literal(3.into())),
            Push(Literal(4.into())),
            StandardOp("MUL".into()),
        ];
        constant_propagation(&mut code);
        assert_eq!(code, vec![Push(Literal(12.into()))]);
    }

    #[test]
    fn test_constant_propagation_div() {
        let mut code = vec![
            Push(Literal(3.into())),
            Push(Literal(8.into())),
            StandardOp("DIV".into()),
        ];
        constant_propagation(&mut code);
        assert_eq!(code, vec![Push(Literal(2.into()))]);
    }

    #[test]
    fn test_constant_propagation_div_zero() {
        let mut code = vec![
            Push(Literal(0.into())),
            Push(Literal(1.into())),
            StandardOp("DIV".into()),
        ];
        constant_propagation(&mut code);
        assert_eq!(code, vec![Push(Literal(0.into()))]);
    }

    #[test]
    fn test_no_op_jump() {
        let mut code = vec![
            Push(Label("mylabel".into())),
            StandardOp("JUMP".into()),
            LocalLabelDeclaration("mylabel".into()),
        ];
        no_op_jumps(&mut code);
        assert_eq!(code, vec![LocalLabelDeclaration("mylabel".into())]);
    }

    #[test]
    fn test_remove_swap() {
        let mut code = vec![
            Push(Literal("42".into())),
            Push(Label("mylabel".into())),
            StandardOp("SWAP1".into()),
        ];
        remove_swaps(&mut code);
        assert_eq!(
            code,
            vec![Push(Label("mylabel".into())), Push(Literal("42".into()))]
        );
    }

    #[test]
    fn test_remove_push_pop() {
        let mut code = vec![Push(Literal("42".into())), StandardOp("POP".into())];
        remove_ignored_values(&mut code);
        assert_eq!(code, vec![]);
    }

    #[test]
    fn test_remove_dup_pop() {
        let mut code = vec![StandardOp("DUP5".into()), StandardOp("POP".into())];
        remove_ignored_values(&mut code);
        assert_eq!(code, vec![]);
    }
}
