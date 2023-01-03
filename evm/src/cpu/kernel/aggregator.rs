//! Loads each kernel assembly file and concatenates them.

use itertools::Itertools;
use once_cell::sync::Lazy;

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::constants::evm_constants;
use crate::cpu::kernel::parser::parse;

pub static KERNEL: Lazy<Kernel> = Lazy::new(combined_kernel);

pub(crate) fn combined_kernel() -> Kernel {
    let files = vec![
        include_str!("asm/bignum/add.asm"),
        include_str!("asm/bignum/ge.asm"),
        include_str!("asm/bignum/mul.asm"),
        include_str!("asm/core/bootloader.asm"),
        include_str!("asm/core/create.asm"),
        include_str!("asm/core/create_addresses.asm"),
        include_str!("asm/core/intrinsic_gas.asm"),
        include_str!("asm/core/invalid.asm"),
        include_str!("asm/core/jumpdest_analysis.asm"),
        include_str!("asm/core/nonce.asm"),
        include_str!("asm/core/process_txn.asm"),
        include_str!("asm/core/syscall.asm"),
        include_str!("asm/core/syscall_stubs.asm"),
        include_str!("asm/core/terminate.asm"),
        include_str!("asm/core/transfer.asm"),
        include_str!("asm/core/util.asm"),
        include_str!("asm/curve/bn254/curve_add.asm"),
        include_str!("asm/curve/bn254/curve_mul.asm"),
        include_str!("asm/curve/bn254/moddiv.asm"),
        include_str!("asm/curve/common.asm"),
        include_str!("asm/curve/secp256k1/curve_add.asm"),
        include_str!("asm/curve/secp256k1/ecrecover.asm"),
        include_str!("asm/curve/secp256k1/inverse_scalar.asm"),
        include_str!("asm/curve/secp256k1/lift_x.asm"),
        include_str!("asm/curve/secp256k1/moddiv.asm"),
        include_str!("asm/curve/secp256k1/glv.asm"),
        include_str!("asm/curve/secp256k1/precomputation.asm"),
        include_str!("asm/exp.asm"),
        include_str!("asm/fields/fp6_macros.asm"),
        include_str!("asm/fields/fp6_mul.asm"),
        include_str!("asm/fields/fp12_mul.asm"),
        include_str!("asm/halt.asm"),
        include_str!("asm/hash/blake2b/addresses.asm"),
        include_str!("asm/hash/blake2b/compression.asm"),
        include_str!("asm/hash/blake2b/g_functions.asm"),
        include_str!("asm/hash/blake2b/hash.asm"),
        include_str!("asm/hash/blake2b/iv.asm"),
        include_str!("asm/hash/blake2b/ops.asm"),
        include_str!("asm/hash/blake2b/permutations.asm"),
        include_str!("asm/hash/blake2b/store.asm"),
        include_str!("asm/hash/ripemd/box.asm"),
        include_str!("asm/hash/ripemd/compression.asm"),
        include_str!("asm/hash/ripemd/constants.asm"),
        include_str!("asm/hash/ripemd/functions.asm"),
        include_str!("asm/hash/ripemd/main.asm"),
        include_str!("asm/hash/ripemd/memory.asm"),
        include_str!("asm/hash/ripemd/update.asm"),
        include_str!("asm/hash/sha2/compression.asm"),
        include_str!("asm/hash/sha2/constants.asm"),
        include_str!("asm/hash/sha2/message_schedule.asm"),
        include_str!("asm/hash/sha2/ops.asm"),
        include_str!("asm/hash/sha2/store_pad.asm"),
        include_str!("asm/hash/sha2/temp_words.asm"),
        include_str!("asm/hash/sha2/write_length.asm"),
        include_str!("asm/main.asm"),
        include_str!("asm/memory/core.asm"),
        include_str!("asm/memory/memcpy.asm"),
        include_str!("asm/memory/metadata.asm"),
        include_str!("asm/memory/packing.asm"),
        include_str!("asm/memory/syscalls.asm"),
        include_str!("asm/memory/txn_fields.asm"),
        include_str!("asm/mpt/accounts.asm"),
        include_str!("asm/mpt/delete/delete.asm"),
        include_str!("asm/mpt/hash/hash.asm"),
        include_str!("asm/mpt/hash/hash_trie_specific.asm"),
        include_str!("asm/mpt/hex_prefix.asm"),
        include_str!("asm/mpt/insert/insert.asm"),
        include_str!("asm/mpt/insert/insert_extension.asm"),
        include_str!("asm/mpt/insert/insert_leaf.asm"),
        include_str!("asm/mpt/insert/insert_trie_specific.asm"),
        include_str!("asm/mpt/load/load.asm"),
        include_str!("asm/mpt/load/load_trie_specific.asm"),
        include_str!("asm/mpt/read.asm"),
        include_str!("asm/mpt/storage/storage_read.asm"),
        include_str!("asm/mpt/storage/storage_write.asm"),
        include_str!("asm/mpt/util.asm"),
        include_str!("asm/rlp/decode.asm"),
        include_str!("asm/rlp/encode.asm"),
        include_str!("asm/rlp/encode_rlp_string.asm"),
        include_str!("asm/rlp/num_bytes.asm"),
        include_str!("asm/rlp/read_to_memory.asm"),
        include_str!("asm/shift.asm"),
        include_str!("asm/transactions/router.asm"),
        include_str!("asm/transactions/type_0.asm"),
        include_str!("asm/transactions/type_1.asm"),
        include_str!("asm/transactions/type_2.asm"),
        include_str!("asm/util/assertions.asm"),
        include_str!("asm/util/basic_macros.asm"),
        include_str!("asm/util/keccak.asm"),
        include_str!("asm/account_code.asm"),
        include_str!("asm/balance.asm"),
    ];

    let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
    assemble(parsed_files, evm_constants(), true)
}

#[cfg(test)]
mod tests {
    use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
    use log::debug;

    use crate::cpu::kernel::aggregator::combined_kernel;

    #[test]
    fn make_kernel() {
        let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "debug"));

        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        debug!("Total kernel size: {} bytes", kernel.code.len());
    }
}
