//! Loads each kernel assembly file and concatenates them.

use itertools::Itertools;

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::parser::parse;

#[allow(dead_code)] // TODO: Should be used once witness generation is done.
pub(crate) fn combined_kernel() -> Kernel {
    let files = vec![
        include_str!("asm/basic_macros.asm"),
        include_str!("asm/exp.asm"),
        include_str!("asm/storage_read.asm"),
        include_str!("asm/storage_write.asm"),
    ];

    let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
    assemble(parsed_files)
}

#[cfg(test)]
mod tests {
    use crate::cpu::kernel::aggregator::combined_kernel;

    #[test]
    fn make_kernel() {
        // Make sure we can parse and assemble the entire kernel.
        combined_kernel();
    }
}
