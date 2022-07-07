//! Loads each kernel assembly file and concatenates them.

use std::collections::HashMap;

use ethereum_types::U256;
use itertools::Itertools;

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::parser::parse;

pub fn evm_constants() -> HashMap<String, U256> {
    let mut c = HashMap::new();
    c.insert("SEGMENT_ID_TXN_DATA".into(), 0.into()); // TODO: Replace with actual segment ID.
    c
}

#[allow(dead_code)] // TODO: Should be used once witness generation is done.
pub(crate) fn combined_kernel() -> Kernel {
    let files = vec![
        include_str!("asm/basic_macros.asm"),
        include_str!("asm/exp.asm"),
        include_str!("asm/curve_mul.asm"),
        include_str!("asm/curve_add.asm"),
        include_str!("asm/moddiv.asm"),
        include_str!("asm/storage_read.asm"),
        include_str!("asm/storage_write.asm"),
    ];

    let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
    assemble(parsed_files, evm_constants())
}

#[cfg(test)]
mod tests {
    use crate::cpu::kernel::aggregator::combined_kernel;

    #[test]
    fn make_kernel() {
        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        println!("Kernel size: {} bytes", kernel.code.len());
    }
}
