//! Loads each kernel function assembly file and concatenates them.

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::ast::Function;
use crate::cpu::kernel::parser::parse;

fn combined_asm() -> String {
    let mut combined = String::new();
    combined.push_str(include_str!("functions/storage_read.asm"));
    combined.push_str(include_str!("functions/storage_write.asm"));
    combined
}

fn combined_ast() -> Vec<Function> {
    parse(&combined_asm())
}

pub fn combined_kernel() -> Kernel {
    assemble(combined_ast())
}

#[cfg(test)]
mod tests {
    use crate::cpu::kernel::aggregator::combined_kernel;

    #[test]
    fn make_kernel() {
        // Make sure we can parse and assemble the entire kernel.
        dbg!(combined_kernel());
    }
}
