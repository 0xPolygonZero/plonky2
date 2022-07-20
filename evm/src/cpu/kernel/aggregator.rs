//! Loads each kernel assembly file and concatenates them.

use std::collections::HashMap;

use ethereum_types::U256;
use hex_literal::hex;
use itertools::Itertools;
use once_cell::sync::Lazy;

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::parser::parse;
use crate::memory::segments::Segment;

pub static KERNEL: Lazy<Kernel> = Lazy::new(combined_kernel);

pub fn evm_constants() -> HashMap<String, U256> {
    let mut c = HashMap::new();
    c.insert(
        "BN_BASE".into(),
        U256::from_big_endian(&hex!(
            "30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"
        )),
    );
    for segment in Segment::all() {
        c.insert(segment.var_name().into(), (segment as u32).into());
    }
    c
}

#[allow(dead_code)] // TODO: Should be used once witness generation is done.
pub(crate) fn combined_kernel() -> Kernel {
    let files = vec![
        include_str!("asm/assertions.asm"),
        include_str!("asm/basic_macros.asm"),
        include_str!("asm/exp.asm"),
        include_str!("asm/curve_mul.asm"),
        include_str!("asm/curve_add.asm"),
        include_str!("asm/memory.asm"),
        include_str!("asm/moddiv.asm"),
        include_str!("asm/secp256k1/curve_mul.asm"),
        include_str!("asm/secp256k1/curve_add.asm"),
        include_str!("asm/secp256k1/moddiv.asm"),
        include_str!("asm/secp256k1/lift_x.asm"),
        include_str!("asm/secp256k1/inverse_scalar.asm"),
        include_str!("asm/ecrecover.asm"),
        include_str!("asm/storage_read.asm"),
        include_str!("asm/storage_write.asm"),
    ];

    let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
    assemble(parsed_files, evm_constants())
}

#[cfg(test)]
mod tests {
    use log::debug;

    use crate::cpu::kernel::aggregator::combined_kernel;

    #[test]
    fn make_kernel() {
        let _ = env_logger::Builder::from_default_env()
            .format_timestamp(None)
            .try_init();

        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        debug!("Total kernel size: {} bytes", kernel.code.len());
    }
}
