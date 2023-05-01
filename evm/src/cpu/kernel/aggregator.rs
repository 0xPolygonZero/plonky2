//! Loads each kernel assembly file and concatenates them.

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use itertools::Itertools;
use once_cell::sync::Lazy;

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::constants::evm_constants;
use crate::cpu::kernel::parser::parse;

pub static KERNEL: Lazy<Kernel> = Lazy::new(combined_kernel);

pub(crate) fn combined_kernel() -> Kernel {
    let files = vec![
        // "global jumped_to_0: PANIC",
        // "global jumped_to_1: PANIC",
        "asm/bignum/add.asm",
        "asm/bignum/addmul.asm",
        "asm/bignum/cmp.asm",
        "asm/bignum/isone.asm",
        "asm/bignum/iszero.asm",
        "asm/bignum/modexp.asm",
        "asm/bignum/modmul.asm",
        "asm/bignum/mul.asm",
        "asm/bignum/shr.asm",
        "asm/bignum/util.asm",
        "asm/core/bootloader.asm",
        "asm/core/call.asm",
        "asm/core/create.asm",
        "asm/core/create_addresses.asm",
        "asm/core/create_contract_account.asm",
        "asm/core/gas.asm",
        "asm/core/intrinsic_gas.asm",
        "asm/core/invalid.asm",
        "asm/core/jumpdest_analysis.asm",
        "asm/core/nonce.asm",
        "asm/core/process_txn.asm",
        "asm/core/syscall.asm",
        "asm/core/syscall_stubs.asm",
        "asm/core/terminate.asm",
        "asm/core/transfer.asm",
        "asm/core/util.asm",
        "asm/core/access_lists.asm",
        "asm/core/selfdestruct_list.asm",
        "asm/core/precompiles/main.asm",
        "asm/core/precompiles/ecrec.asm",
        "asm/core/precompiles/sha256.asm",
        "asm/core/precompiles/rip160.asm",
        "asm/core/precompiles/id.asm",
        "asm/core/precompiles/expmod.asm",
        "asm/core/precompiles/bn_add.asm",
        "asm/core/precompiles/bn_mul.asm",
        "asm/core/precompiles/snarkv.asm",
        "asm/core/precompiles/blake2_f.asm",
        "asm/curve/bls381/util.asm",
        "asm/curve/bn254/curve_arithmetic/constants.asm",
        "asm/curve/bn254/curve_arithmetic/curve_add.asm",
        "asm/curve/bn254/curve_arithmetic/curve_mul.asm",
        "asm/curve/bn254/curve_arithmetic/final_exponent.asm",
        "asm/curve/bn254/curve_arithmetic/glv.asm",
        "asm/curve/bn254/curve_arithmetic/miller_loop.asm",
        "asm/curve/bn254/curve_arithmetic/msm.asm",
        "asm/curve/bn254/curve_arithmetic/pairing.asm",
        "asm/curve/bn254/curve_arithmetic/precomputation.asm",
        "asm/curve/bn254/curve_arithmetic/twisted_curve.asm",
        "asm/curve/bn254/field_arithmetic/degree_6_mul.asm",
        "asm/curve/bn254/field_arithmetic/degree_12_mul.asm",
        "asm/curve/bn254/field_arithmetic/frobenius.asm",
        "asm/curve/bn254/field_arithmetic/inverse.asm",
        "asm/curve/bn254/field_arithmetic/util.asm",
        "asm/curve/common.asm",
        "asm/curve/secp256k1/curve_add.asm",
        "asm/curve/secp256k1/ecrecover.asm",
        "asm/curve/secp256k1/inverse_scalar.asm",
        "asm/curve/secp256k1/lift_x.asm",
        "asm/curve/secp256k1/moddiv.asm",
        "asm/curve/secp256k1/glv.asm",
        "asm/curve/secp256k1/precomputation.asm",
        "asm/curve/wnaf.asm",
        "asm/exp.asm",
        "asm/halt.asm",
        "asm/hash/blake2/addresses.asm",
        "asm/hash/blake2/blake2_f.asm",
        // "asm/hash/blake2/blake2b.asm",
        // "asm/hash/blake2/compression.asm",
        "asm/hash/blake2/g_functions.asm",
        "asm/hash/blake2/hash.asm",
        "asm/hash/blake2/iv.asm",
        "asm/hash/blake2/ops.asm",
        "asm/hash/blake2/permutations.asm",
        "asm/hash/ripemd/box.asm",
        "asm/hash/ripemd/compression.asm",
        "asm/hash/ripemd/constants.asm",
        "asm/hash/ripemd/functions.asm",
        "asm/hash/ripemd/main.asm",
        "asm/hash/ripemd/update.asm",
        "asm/hash/sha2/compression.asm",
        "asm/hash/sha2/constants.asm",
        "asm/hash/sha2/main.asm",
        "asm/hash/sha2/message_schedule.asm",
        "asm/hash/sha2/ops.asm",
        "asm/hash/sha2/temp_words.asm",
        "asm/hash/sha2/write_length.asm",
        "asm/main.asm",
        "asm/memory/core.asm",
        "asm/memory/memcpy.asm",
        "asm/memory/memset.asm",
        "asm/memory/metadata.asm",
        "asm/memory/packing.asm",
        "asm/memory/syscalls.asm",
        "asm/memory/txn_fields.asm",
        "asm/mpt/accounts.asm",
        "asm/mpt/delete/delete.asm",
        "asm/mpt/hash/hash.asm",
        "asm/mpt/hash/hash_trie_specific.asm",
        "asm/mpt/hex_prefix.asm",
        "asm/mpt/insert/insert.asm",
        "asm/mpt/insert/insert_extension.asm",
        "asm/mpt/insert/insert_leaf.asm",
        "asm/mpt/insert/insert_trie_specific.asm",
        "asm/mpt/load/load.asm",
        "asm/mpt/load/load_trie_specific.asm",
        "asm/mpt/read.asm",
        "asm/mpt/storage/storage_read.asm",
        "asm/mpt/storage/storage_write.asm",
        "asm/mpt/util.asm",
        "asm/rlp/decode.asm",
        "asm/rlp/encode.asm",
        "asm/rlp/encode_rlp_scalar.asm",
        "asm/rlp/encode_rlp_string.asm",
        "asm/rlp/num_bytes.asm",
        "asm/rlp/read_to_memory.asm",
        "asm/shift.asm",
        "asm/signed.asm",
        "asm/transactions/common_decoding.asm",
        "asm/transactions/router.asm",
        "asm/transactions/type_0.asm",
        "asm/transactions/type_1.asm",
        "asm/transactions/type_2.asm",
        "asm/util/assertions.asm",
        "asm/util/basic_macros.asm",
        "asm/util/keccak.asm",
        "asm/account_code.asm",
        "asm/balance.asm",
    ];

    // let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
    // assemble(parsed_files, evm_constants(), true)
    let parsed_files = files
        .iter()
        .map(|fp| {
            let mut file_content = String::new();
            let mut path = PathBuf::from("/Users/wborgeaud/Mir/plonky2/evm/src/cpu/kernel/");
            path.push(fp);
            let mut file = File::open(path).expect(fp);
            file.read_to_string(&mut file_content).unwrap();
            parse(&file_content)
        })
        .collect_vec();

    // let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
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
