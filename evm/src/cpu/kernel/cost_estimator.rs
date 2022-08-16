use crate::cpu::kernel::assembler::BYTES_PER_OFFSET;
use crate::cpu::kernel::ast::Item;
use crate::cpu::kernel::ast::Item::*;
use crate::cpu::kernel::ast::PushTarget::*;
use crate::cpu::kernel::utils::u256_to_trimmed_be_bytes;

pub(crate) fn is_code_improved(before: &[Item], after: &[Item]) -> bool {
    cost_estimate(after) < cost_estimate(before)
}

fn cost_estimate(code: &[Item]) -> u32 {
    code.iter().map(cost_estimate_item).sum()
}

fn cost_estimate_item(item: &Item) -> u32 {
    match item {
        MacroDef(_, _, _) => 0,
        GlobalLabelDeclaration(_) => 0,
        LocalLabelDeclaration(_) => 0,
        Push(Literal(n)) => cost_estimate_push(u256_to_trimmed_be_bytes(n).len()),
        Push(Label(_)) => cost_estimate_push(BYTES_PER_OFFSET as usize),
        ProverInput(_) => 1,
        StandardOp(op) => cost_estimate_standard_op(op.as_str()),
        _ => panic!("Unexpected item: {:?}", item),
    }
}

fn cost_estimate_standard_op(_op: &str) -> u32 {
    // For now we just treat any standard operation as having the same cost. This is pretty naive,
    // but should work fine with our current set of simple optimization rules.
    1
}

fn cost_estimate_push(num_bytes: usize) -> u32 {
    // TODO: Once PUSH is actually implemented, check if this needs to be revised.
    num_bytes as u32
}
