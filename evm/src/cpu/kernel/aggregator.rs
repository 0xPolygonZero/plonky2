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
    use std::str::FromStr;

    use anyhow::Result;
    use ethereum_types::U256;
    use log::debug;
    use rand::{thread_rng, Rng};

    use crate::cpu::kernel::aggregator::combined_kernel;
    use crate::cpu::kernel::interpreter::run;

    #[test]
    fn make_kernel() {
        let _ = env_logger::Builder::from_default_env()
            .format_timestamp(None)
            .try_init();

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
