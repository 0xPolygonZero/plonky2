#[cfg(test)]
mod bn {
    use anyhow::Result;
    use ethereum_types::U256;

    use crate::cpu::kernel::aggregator::combined_kernel;
    use crate::cpu::kernel::interpreter::run_with_kernel;
    use crate::cpu::kernel::tests::u256ify;

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
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);
        // Standard addition #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);

        // Standard doubling #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #2
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_double, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #3
        let initial_stack = u256ify(["0xdeadbeef", "0x2", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);

        // Addition with identity #1
        let initial_stack = u256ify(["0xdeadbeef", identity.1, identity.0, point1.1, point1.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, identity.1, identity.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #3
        let initial_stack =
            u256ify(["0xdeadbeef", identity.1, identity.0, identity.1, identity.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);

        // Addition with invalid point(s) #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, invalid.1, invalid.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #2
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #3
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, identity.1, identity.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #4
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, invalid.1, invalid.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);

        // Scalar multiplication #1
        let initial_stack = u256ify(["0xdeadbeef", s, point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point4.1, point4.0])?);
        // Scalar multiplication #2
        let initial_stack = u256ify(["0xdeadbeef", "0x0", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);
        // Scalar multiplication #3
        let initial_stack = u256ify(["0xdeadbeef", "0x1", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point0.1, point0.0])?);
        // Scalar multiplication #4
        let initial_stack = u256ify(["0xdeadbeef", s, identity.1, identity.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);
        // Scalar multiplication #5
        let initial_stack = u256ify(["0xdeadbeef", s, invalid.1, invalid.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);

        // Multiple calls
        let ec_mul_hex = format!("0x{ec_mul:x}");
        let initial_stack = u256ify([
            "0xdeadbeef",
            s,
            &ec_mul_hex,
            identity.1,
            identity.0,
            point0.1,
            point0.0,
        ])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point4.1, point4.0])?);

        Ok(())
    }
}

#[cfg(test)]
mod secp {
    use anyhow::Result;

    use crate::cpu::kernel::aggregator::combined_kernel;
    use crate::cpu::kernel::interpreter::{run, run_with_kernel};
    use crate::cpu::kernel::tests::u256ify;

    #[test]
    fn test_ec_ops() -> Result<()> {
        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        let ec_add = kernel.global_labels["ec_add_valid_points_secp"];
        let ec_double = kernel.global_labels["ec_double_secp"];
        let ec_mul = kernel.global_labels["ec_mul_valid_point_secp"];
        let identity = ("0x0", "0x0");
        let point0 = (
            "0xc82ccceebd739e646631b7270ed8c33e96c4940b19db91eaf67da6ec92d109b",
            "0xe0d241d2de832656c3eed78271bb06b5602d6473742c7c48a38b9f0350a76164",
        );
        let point1 = (
            "0xbf26b1a7a46025d0a1787aa050d0bb83b8a4746010f873404389b8b23360919c",
            "0x65adeff3fed1b22fa10279b5a25b96694a20bcbf6b718c0412f6d34a2e9bb924",
        );
        // point2 = point0 + point1
        let point2 = (
            "0x191e8183402c6d6f5f22a9fe2a5ce17a7dd5184bd5d359c77189e9f714a18225",
            "0xe23fbb6913de7449d92e4dfbe278e2874fac80d53bfeb8fb3400462b7bfaec74",
        );
        // point3 = 2 * point0
        let point3 = (
            "0x7872498939b02197c2b6f0a0f5767f36551e43f910de472fbbff0538b21f5f45",
            "0x294e15025d935438023a0e4056892abd6405fade13cf2b3131d8755be7cebad",
        );
        let s = "0xa72ad7d8ce24135b5138f853d7a9896381c40523b5d1cf03072151f2af10e35e";
        // point4 = s * point0
        let point4 = (
            "0xd8bec38864f0fe56d429540e6de624afb8ddc7fba1f738337913922a30b96c14",
            "0x5b086b2720ac39d173777bc36a49629c80c3a3e55e1c50527e60016d9be71318",
        );

        // Standard addition #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point1.1, point1.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);
        // Standard addition #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack, &kernel.prover_inputs)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);

        // Standard doubling #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #2
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_double, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #3
        let initial_stack = u256ify(["0xdeadbeef", "0x2", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);

        // Addition with identity #1
        let initial_stack = u256ify(["0xdeadbeef", identity.1, identity.0, point1.1, point1.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, identity.1, identity.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #3
        let initial_stack =
            u256ify(["0xdeadbeef", identity.1, identity.0, identity.1, identity.0])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);

        // Scalar multiplication #1
        let initial_stack = u256ify(["0xdeadbeef", s, point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point4.1, point4.0])?);
        // Scalar multiplication #2
        let initial_stack = u256ify(["0xdeadbeef", "0x0", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);
        // Scalar multiplication #3
        let initial_stack = u256ify(["0xdeadbeef", "0x1", point0.1, point0.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point0.1, point0.0])?);
        // Scalar multiplication #4
        let initial_stack = u256ify(["0xdeadbeef", s, identity.1, identity.0])?;
        let stack = run_with_kernel(&kernel, ec_mul, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);

        // Multiple calls
        let ec_mul_hex = format!("0x{ec_mul:x}");
        let initial_stack = u256ify([
            "0xdeadbeef",
            s,
            &ec_mul_hex,
            identity.1,
            identity.0,
            point0.1,
            point0.0,
        ])?;
        let stack = run_with_kernel(&kernel, ec_add, initial_stack)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point4.1, point4.0])?);

        Ok(())
    }
}
