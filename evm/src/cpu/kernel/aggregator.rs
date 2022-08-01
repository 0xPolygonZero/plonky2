//! Loads each kernel assembly file and concatenates them.

use itertools::Itertools;
use once_cell::sync::Lazy;

use super::assembler::{assemble, Kernel};
use crate::cpu::kernel::constants::evm_constants;
use crate::cpu::kernel::parser::parse;

pub static KERNEL: Lazy<Kernel> = Lazy::new(combined_kernel);

pub(crate) fn combined_kernel() -> Kernel {
    let files = vec![
        include_str!("asm/assertions.asm"),
        include_str!("asm/basic_macros.asm"),
        include_str!("asm/core/bootloader.asm"),
        include_str!("asm/core/create.asm"),
        include_str!("asm/core/create_addresses.asm"),
        include_str!("asm/core/intrinsic_gas.asm"),
        include_str!("asm/core/invalid.asm"),
        include_str!("asm/core/nonce.asm"),
        include_str!("asm/core/process_txn.asm"),
        include_str!("asm/core/terminate.asm"),
        include_str!("asm/core/transfer.asm"),
        include_str!("asm/core/util.asm"),
        include_str!("asm/curve/bn254/curve_add.asm"),
        include_str!("asm/curve/bn254/curve_mul.asm"),
        include_str!("asm/curve/bn254/moddiv.asm"),
        include_str!("asm/curve/common.asm"),
        include_str!("asm/curve/secp256k1/curve_mul.asm"),
        include_str!("asm/curve/secp256k1/curve_add.asm"),
        include_str!("asm/curve/secp256k1/ecrecover.asm"),
        include_str!("asm/curve/secp256k1/inverse_scalar.asm"),
        include_str!("asm/curve/secp256k1/lift_x.asm"),
        include_str!("asm/curve/secp256k1/moddiv.asm"),
        include_str!("asm/halt.asm"),
        include_str!("asm/main.asm"),
        include_str!("asm/memory/core.asm"),
        include_str!("asm/memory/memcpy.asm"),
        include_str!("asm/memory/metadata.asm"),
        include_str!("asm/memory/packing.asm"),
        include_str!("asm/memory/txn_fields.asm"),
        include_str!("asm/exp.asm"),
        include_str!("asm/helper_functions.asm"),
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
        include_str!("asm/mpt/hash.asm"),
        include_str!("asm/mpt/hash_trie_specific.asm"),
        include_str!("asm/mpt/hex_prefix.asm"),
        include_str!("asm/mpt/load.asm"),
        include_str!("asm/mpt/read.asm"),
        include_str!("asm/mpt/storage_read.asm"),
        include_str!("asm/mpt/storage_write.asm"),
        include_str!("asm/mpt/util.asm"),
        include_str!("asm/mpt/write.asm"),
        include_str!("asm/transactions/router.asm"),
        include_str!("asm/transactions/type_0.asm"),
        include_str!("asm/transactions/type_1.asm"),
        include_str!("asm/transactions/type_2.asm"),
        include_str!("asm/util/assertions.asm"),
        include_str!("asm/util/basic_macros.asm"),
    ];

    let parsed_files = files.iter().map(|f| parse(f)).collect_vec();
    assemble(parsed_files, evm_constants(), true)
}

#[cfg(test)]
mod tests {
    use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
    use std::str::FromStr;

    use anyhow::Result;
    use ethereum_types::U256;
    use log::debug;
    use rand::thread_rng;

    use crate::cpu::kernel::{aggregator::combined_kernel, interpreter::run};

    #[test]
    fn make_kernel() {
        let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "debug"));

        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        debug!("Total kernel size: {} bytes", kernel.code.len());
    }

    fn u256ify<'a>(hexes: impl IntoIterator<Item = &'a str>) -> Result<Vec<U256>> {
        Ok(hexes
            .into_iter()
            .map(U256::from_str)
            .collect::<Result<Vec<_>, _>>()?)
    }

    #[test]
    fn test_insert() -> Result<()> {
        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        let exp = kernel.global_labels["swapn"];
        let mut rng = thread_rng();
        let a = U256([0; 4].map(|_| rng.gen()));
        let b = U256([0; 4].map(|_| rng.gen()));
        let n = rng.gen_range(0..16);
        let n_u256 = U256([n, 0, 0, 0]);

        let mut initial_stack = vec![U256::from_str("0xdeadbeef")?, n_u256, b];
        initial_stack.extend([a; 16]);
        let stack_with_kernel = run(&kernel.code, exp, initial_stack);

        dbg!(stack_with_kernel);
        let expected_stack = todo!();

        // assert_eq!(stack_with_kernel, expected_stack);

        Ok(())
    }

    #[test]
    fn test_exp() -> Result<()> {
        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        let exp = kernel.global_labels["exp"];
        let mut rng = thread_rng();
        let a = U256([0; 4].map(|_| rng.gen()));
        let b = U256([0; 4].map(|_| rng.gen()));

        // Random input
        let initial_stack = vec![U256::from_str("0xdeadbeef")?, b, a];
        let stack_with_kernel = run(&kernel.code, exp, initial_stack);
        let initial_stack = vec![b, a];
        let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
        let stack_with_opcode = run(&code, 0, initial_stack);
        assert_eq!(stack_with_kernel, stack_with_opcode);

        // 0 base
        let initial_stack = vec![U256::from_str("0xdeadbeef")?, b, U256::zero()];
        let stack_with_kernel = run(&kernel.code, exp, initial_stack);
        let initial_stack = vec![b, U256::zero()];
        let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
        let stack_with_opcode = run(&code, 0, initial_stack);
        assert_eq!(stack_with_kernel, stack_with_opcode);

        // 0 exponent
        let initial_stack = vec![U256::from_str("0xdeadbeef")?, U256::zero(), a];
        let stack_with_kernel = run(&kernel.code, exp, initial_stack);
        let initial_stack = vec![U256::zero(), a];
        let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
        let stack_with_opcode = run(&code, 0, initial_stack);
        assert_eq!(stack_with_kernel, stack_with_opcode);

        Ok(())
    }

    #[test]
    fn test_ec_ops() -> Result<()> {
        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        let ec_add = kernel.global_labels["ec_add"];
        let ec_double = kernel.global_labels["ec_double"];
        let ec_mul = kernel.global_labels["ec_mul"];
        let identity = ("0x0", "0x0");
        let invalid = ("0x0", "0x3"); // Not on curve
        let point0 = (
            "0x1feee7ec986e198890cb83be8b8ba09ee953b3f149db6d9bfdaa5c308a33e58d",
            "0x2051cc9a9edd46231604fd88f351e95ec72a285be93e289ac59cb48561efb2c6",
        );
        let point1 = (
            "0x15b64d0a5f329fb672029298be8050f444626e6de11903caffa74b388075be1b",
            "0x2d9e07340bd5cd7b70687b98f2500ff930a89a30d7b6a3e04b1b4d345319d234",
        );
        // point2 = point0 + point1
        let point2 = (
            "0x18659c0e0a8fedcb8747cf463fc7cfa05f667d84e771d0a9521fc1a550688f0c",
            "0x283ed10b42703e187e7a808aeb45c6b457bc4cc7d704e53b3348a1e3b0bfa55b",
        );
        // point3 = 2 * point0
        let point3 = (
            "0x17da2b7b1a01c8dfdf0f5a6415833c7d755d219aa7e2c4cd0ac83d87d0ca4217",
            "0xc9ace9de14aac8114541b50c19320eb40f0eeac3621526d9e34dbcf4c3a6c0f",
        );
        let s = "0xabb2a34c0e7956cfe6cef9ddb7e810c45ea19a6ebadd79c21959af09f5ba480a";
        // point4 = s * point0
        let point4 = (
            "0xe519344959cc17021fe98878f947f5c1b1675325533a620c1684cfa6367e6c0",
            "0x7496a7575b0b6a821e19ce780ecc3e0b156e605327798693defeb9f265b7a6f",
        );

        // Standard addition #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point1.1, point1.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, u256ify([point2.1, point2.0])?);
        // Standard addition #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, u256ify([point2.1, point2.0])?);

        // Standard doubling #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #2
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_double, initial_stack);
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #3
        let initial_stack = u256ify(["0xdeadbeef", "0x2", point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_mul, initial_stack);
        assert_eq!(stack, u256ify([point3.1, point3.0])?);

        // Addition with identity #1
        let initial_stack = u256ify(["0xdeadbeef", identity.1, identity.0, point1.1, point1.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, identity.1, identity.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #3
        let initial_stack =
            u256ify(["0xdeadbeef", identity.1, identity.0, identity.1, identity.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, u256ify([identity.1, identity.0])?);

        // Addition with invalid point(s) #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, invalid.1, invalid.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #2
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #3
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, identity.1, identity.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #4
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, invalid.1, invalid.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);

        // Scalar multiplication #1
        let initial_stack = u256ify(["0xdeadbeef", s, point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_mul, initial_stack);
        assert_eq!(stack, u256ify([point4.1, point4.0])?);
        // Scalar multiplication #2
        let initial_stack = u256ify(["0xdeadbeef", "0x0", point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_mul, initial_stack);
        assert_eq!(stack, u256ify([identity.1, identity.0])?);
        // Scalar multiplication #3
        let initial_stack = u256ify(["0xdeadbeef", "0x1", point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_mul, initial_stack);
        assert_eq!(stack, u256ify([point0.1, point0.0])?);
        // Scalar multiplication #4
        let initial_stack = u256ify(["0xdeadbeef", s, identity.1, identity.0])?;
        let stack = run(&kernel.code, ec_mul, initial_stack);
        assert_eq!(stack, u256ify([identity.1, identity.0])?);
        // Scalar multiplication #5
        let initial_stack = u256ify(["0xdeadbeef", s, invalid.1, invalid.0])?;
        let stack = run(&kernel.code, ec_mul, initial_stack);
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);

        // Multiple calls
        let ec_mul_hex = format!("0x{:x}", ec_mul);
        let initial_stack = u256ify([
            "0xdeadbeef",
            s,
            &ec_mul_hex,
            identity.1,
            identity.0,
            point0.1,
            point0.0,
        ])?;
        let stack = run(&kernel.code, ec_add, initial_stack);
        assert_eq!(stack, u256ify([point4.1, point4.0])?);

        Ok(())
    }
}
