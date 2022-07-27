//! Loads each kernel assembly file and concatenates them.

use std::collections::HashMap;

use ethereum_types::U256;
use hex_literal::hex;
use itertools::Itertools;
use once_cell::sync::Lazy;

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::parser::parse;
use crate::cpu::kernel::txn_fields::NormalizedTxnField;
use crate::memory::segments::Segment;

pub static KERNEL: Lazy<Kernel> = Lazy::new(combined_kernel);

const EC_CONSTANTS: [(&str, [u8; 32]); 3] = [
    (
        "BN_BASE",
        hex!("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47"),
    ),
    (
        "SECP_BASE",
        hex!("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f"),
    ),
    (
        "SECP_SCALAR",
        hex!("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141"),
    ),
];

pub fn evm_constants() -> HashMap<String, U256> {
    let mut c = HashMap::new();
    for (name, value) in EC_CONSTANTS {
        c.insert(name.into(), U256::from_big_endian(&value));
    }
    for segment in Segment::all() {
        c.insert(segment.var_name().into(), (segment as u32).into());
    }
    for txn_field in NormalizedTxnField::all() {
        c.insert(txn_field.var_name().into(), (txn_field as u32).into());
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
        include_str!("asm/rlp/encode.asm"),
        include_str!("asm/rlp/decode.asm"),
        include_str!("asm/rlp/read_to_memory.asm"),
        include_str!("asm/storage/read.asm"),
        include_str!("asm/storage/write.asm"),
        include_str!("asm/transactions/process_normalized.asm"),
        include_str!("asm/transactions/router.asm"),
        include_str!("asm/transactions/type_0.asm"),
        include_str!("asm/transactions/type_1.asm"),
        include_str!("asm/transactions/type_2.asm"),
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
