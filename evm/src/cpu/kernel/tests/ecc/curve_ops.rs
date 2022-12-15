#[cfg(test)]
mod bn {
    use anyhow::Result;
    use ethereum_types::U256;

    use crate::cpu::kernel::aggregator::KERNEL;
    use crate::cpu::kernel::interpreter::{run_interpreter, Interpreter};
    use crate::cpu::kernel::tests::u256ify;
    use crate::memory::segments::Segment;

    #[test]
    fn test_ec_ops() -> Result<()> {
        // Make sure we can parse and assemble the entire kernel.
        let ec_add = KERNEL.global_labels["ec_add"];
        let ec_double = KERNEL.global_labels["ec_double"];
        let ec_mul = KERNEL.global_labels["ec_mul"];
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
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);
        // Standard addition #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, point0.1, point0.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);

        // Standard doubling #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point0.1, point0.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #2
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0])?;
        let stack = run_interpreter(ec_double, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #3
        let initial_stack = u256ify(["0xdeadbeef", "0x2", point0.1, point0.0])?;
        let stack = run_interpreter(ec_mul, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);

        // Addition with identity #1
        let initial_stack = u256ify(["0xdeadbeef", identity.1, identity.0, point1.1, point1.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, identity.1, identity.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #3
        let initial_stack =
            u256ify(["0xdeadbeef", identity.1, identity.0, identity.1, identity.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);

        // Addition with invalid point(s) #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, invalid.1, invalid.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #2
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, point0.1, point0.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #3
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, identity.1, identity.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);
        // Addition with invalid point(s) #4
        let initial_stack = u256ify(["0xdeadbeef", invalid.1, invalid.0, invalid.1, invalid.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, vec![U256::MAX, U256::MAX]);

        // Scalar multiplication #1
        let initial_stack = u256ify(["0xdeadbeef", s, point0.1, point0.0])?;
        let stack = run_interpreter(ec_mul, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point4.1, point4.0])?);
        // Scalar multiplication #2
        let initial_stack = u256ify(["0xdeadbeef", "0x0", point0.1, point0.0])?;
        let stack = run_interpreter(ec_mul, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);
        // Scalar multiplication #3
        let initial_stack = u256ify(["0xdeadbeef", "0x1", point0.1, point0.0])?;
        let stack = run_interpreter(ec_mul, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point0.1, point0.0])?);
        // Scalar multiplication #4
        let initial_stack = u256ify(["0xdeadbeef", s, identity.1, identity.0])?;
        let stack = run_interpreter(ec_mul, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);
        // Scalar multiplication #5
        let initial_stack = u256ify(["0xdeadbeef", s, invalid.1, invalid.0])?;
        let stack = run_interpreter(ec_mul, initial_stack)?.stack().to_vec();
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
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point4.1, point4.0])?);

        Ok(())
    }

    #[test]
    fn test_glv_verify_data() -> Result<()> {
        let glv = KERNEL.global_labels["bn_glv_decompose"];

        let f = include_str!("bn_glv_test_data");
        for line in f.lines().filter(|s| !s.starts_with("//")) {
            let mut line = line
                .split_whitespace()
                .map(|s| U256::from_str_radix(s, 10).unwrap())
                .collect::<Vec<_>>();
            let k = line.remove(0);
            line.reverse();

            let mut initial_stack = u256ify(["0xdeadbeef"])?;
            initial_stack.push(k);
            let mut int = Interpreter::new(&KERNEL.code, glv, initial_stack, &KERNEL.prover_inputs);
            int.run()?;

            assert_eq!(line, int.stack());
        }

        Ok(())
    }

    #[test]
    fn test_precomputation() -> Result<()> {
        let precompute = KERNEL.global_labels["bn_precompute_table"];

        let initial_stack = u256ify([
            "0xdeadbeef",
            "0x10d7cf0621b6e42c1dbb421f5ef5e1936ca6a87b38198d1935be31e28821d171",
            "0x11b7d55f16aaac07de9a0ed8ac2e8023570dbaa78571fc95e553c4b3ba627689",
        ])?;
        let mut int = Interpreter::new(
            &KERNEL.code,
            precompute,
            initial_stack,
            &KERNEL.prover_inputs,
        );
        int.run()?;

        let mut computed_table = Vec::new();
        for i in 0..32 {
            computed_table.push(
                int.generation_state
                    .memory
                    .mload_general(0, Segment::BnTableQ, i),
            );
        }

        let table = u256ify([
            "0x11b7d55f16aaac07de9a0ed8ac2e8023570dbaa78571fc95e553c4b3ba627689",
            "0x10d7cf0621b6e42c1dbb421f5ef5e1936ca6a87b38198d1935be31e28821d171",
            "0x1565e5587d8566239c23219bc0e1d1d267d19100c3869d0c55b1e3ea4532304e",
            "0x19fd9b572558479df062632562113e4d9a3eb655698ee3be9a5350ed23e690ee",
            "0x19469e55e27021c0af1310ad266cdf1d9eef6942c80afe9c7b517acf16a2a3e1",
            "0x226ec29db9339d7ffb1bc3260f1ca008b804f78553d316c37203118466bb5f5a",
            "0x10a16b4786bd1717a031a1948010593173d36ab35535641c9fe41802d639b435",
            "0x294fe34d7ec9024c96cfde58311b9ee394ff9f8735d882005fcf0d28709b459d",
            "0x300f58e61d4ab1872f6b5fad517c6df1b23468fcfa81154786ec230cb0df6d20",
            "0x12ff1d200127d2ba7a0171cadbe0f729fc5acbe95565cc57f07c9fa42c001390",
            "0x1045a28c9a35a17b63da593c0137ac08a1fda78430b71755941d3dc501b35272",
            "0x2a3f4d91b58179451ec177f599d7eaf79e2555f169fd3e5d2af314600fad299",
            "0x21de5680f03b262f53d3252d5ca71bbc5f2c9ff5483fb63abaea1ee7e9cede1d",
            "0x144249d3fc4c82327845a38ea51181acb374ab30a1e7ea0f13bc8a8b04d96411",
            "0x2ba4ce4289de377397878c1195e21a1d573b02d9463f5c454ec50bdf11aee512",
            "0x259a447b42bab48e07388baece550607bc0a8a88e1ea224eba94c6bed08e470e",
            "0x2ba4ce4289de377397878c1195e21a1d573b02d9463f5c454ec50bdf11aee512",
            "0xaca09f79e76eb9bb117ba07b32c5255db76e0088687a83e818bc55807eeb639",
            "0x21de5680f03b262f53d3252d5ca71bbc5f2c9ff5483fb63abaea1ee7e9cede1d",
            "0x1c22049ee4e51df7400aa227dc6fd6b0e40cbf60c689e07e2864018bd3a39936",
            "0x1045a28c9a35a17b63da593c0137ac08a1fda78430b71755941d3dc501b35272",
            "0x2dc05999c5d9889566642e3727e3d9ae1d9f153251d1f6a769715ad0d7822aae",
            "0x300f58e61d4ab1872f6b5fad517c6df1b23468fcfa81154786ec230cb0df6d20",
            "0x1d653152e009cd6f3e4ed3eba5a061339b269ea8130bfe354ba3ec72ac7ce9b7",
            "0x10a16b4786bd1717a031a1948010593173d36ab35535641c9fe41802d639b435",
            "0x7146b2562689ddd2180675e5065b97a0281cb0a3299488cdc517eee67e1b7aa",
            "0x19469e55e27021c0af1310ad266cdf1d9eef6942c80afe9c7b517acf16a2a3e1",
            "0xdf58bd527fe02a9bd3482907264b854df7c730c149eb3c9ca1d7a9271c19ded",
            "0x1565e5587d8566239c23219bc0e1d1d267d19100c3869d0c55b1e3ea4532304e",
            "0x1666b31bbbd9588bc7ede2911f701a0ffd42b43bfee2e6cea1cd3b29b4966c59",
            "0x11b7d55f16aaac07de9a0ed8ac2e8023570dbaa78571fc95e553c4b3ba627689",
            "0x1f8c7f6cbf7abbfd9a950397228b76ca2adac21630583d7406625a34505b2bd6",
        ])?;

        assert_eq!(computed_table, table);

        Ok(())
    }
}

#[cfg(test)]
mod secp {
    use std::str::FromStr;

    use anyhow::Result;
    use ethereum_types::U256;

    use crate::cpu::kernel::aggregator::{combined_kernel, KERNEL};
    use crate::cpu::kernel::interpreter::{run, run_interpreter, Interpreter};
    use crate::cpu::kernel::tests::u256ify;
    use crate::memory::segments::Segment;

    #[test]
    fn test_ec_ops() -> Result<()> {
        // Make sure we can parse and assemble the entire kernel.
        let kernel = combined_kernel();
        let ec_add = kernel.global_labels["ec_add_valid_points_secp"];
        let ec_double = kernel.global_labels["ec_double_secp"];
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

        // Standard addition #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point1.1, point1.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);
        // Standard addition #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, point0.1, point0.0])?;
        let stack = run(&kernel.code, ec_add, initial_stack, &kernel.prover_inputs)?
            .stack()
            .to_vec();
        assert_eq!(stack, u256ify([point2.1, point2.0])?);

        // Standard doubling #1
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0, point0.1, point0.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);
        // Standard doubling #2
        let initial_stack = u256ify(["0xdeadbeef", point0.1, point0.0])?;
        let stack = run_interpreter(ec_double, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point3.1, point3.0])?);

        // Addition with identity #1
        let initial_stack = u256ify(["0xdeadbeef", identity.1, identity.0, point1.1, point1.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #2
        let initial_stack = u256ify(["0xdeadbeef", point1.1, point1.0, identity.1, identity.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([point1.1, point1.0])?);
        // Addition with identity #3
        let initial_stack =
            u256ify(["0xdeadbeef", identity.1, identity.0, identity.1, identity.0])?;
        let stack = run_interpreter(ec_add, initial_stack)?.stack().to_vec();
        assert_eq!(stack, u256ify([identity.1, identity.0])?);

        Ok(())
    }

    #[test]
    fn test_wnaf() -> Result<()> {
        let wnaf = KERNEL.global_labels["wnaf"];

        let mut initial_stack = u256ify(["0xdeadbeef", "0x1302837587eede977d2814a6906d59bea"])?;
        initial_stack.push((Segment::WnafA as usize).into());
        initial_stack.push(
            U256::from_str("0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
                .unwrap(),
        );
        let mut int = Interpreter::new(&KERNEL.code, wnaf, initial_stack, &KERNEL.prover_inputs);
        int.run()?;

        let mut computed_wnaf = Vec::new();
        for i in 0..130 {
            computed_wnaf.push(
                int.generation_state
                    .memory
                    .mload_general(0, Segment::WnafA, i),
            );
        }

        let mut wnaf = vec![
            0, 21, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 11, 0, 0, 0, 0, 13, 0, 0, 0, 0, 3, 0, 0,
            0, 0, 0, 0, 9, 0, 0, 0, 0, 19, 0, 0, 0, 0, 19, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 5,
            0, 0, 0, 0, 29, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 9, 0, 0, 0, 0, 15, 0, 0, 0, 0, 27, 0,
            0, 0, 0, 0, 31, 0, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 27, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0,
            0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 19, 0, 0, 0, 0, 1,
        ];
        wnaf.reverse();
        let wnaf: Vec<_> = wnaf.into_iter().map(|x| x.into()).collect();
        assert_eq!(computed_wnaf, wnaf);

        Ok(())
    }

    #[test]
    fn test_precomputation() -> Result<()> {
        let precompute = KERNEL.global_labels["precompute_table"];

        let initial_stack = u256ify([
            "0xdeadbeef",
            "0x1573e3e020286de02eed0ab499ea314b9e7a037b6c1ac9ff73dd27f8b59eef1",
            "0xf212ceb1de39d4f60b913e04990ac959c43369da9a6bac46f0e55e4298228aff",
        ])?;
        let mut int = Interpreter::new(
            &KERNEL.code,
            precompute,
            initial_stack,
            &KERNEL.prover_inputs,
        );
        int.run()?;

        let mut computed_table = Vec::new();
        for i in 0..32 {
            computed_table.push(int.generation_state.memory.mload_general(
                0,
                Segment::EcdsaTableQ,
                i,
            ));
        }

        let table = u256ify([
            "0xf212ceb1de39d4f60b913e04990ac959c43369da9a6bac46f0e55e4298228aff",
            "0x1573e3e020286de02eed0ab499ea314b9e7a037b6c1ac9ff73dd27f8b59eef1",
            "0x55c63d63f7302880d113baf54f8e2e954df5a2c2b9c266c88aaedb712bb01581",
            "0xc57ff9008c37ebe8ec5d68e952fbd7543beabdbd167709a836fc618afb37283",
            "0x6f997fb77607bc634239dbfa14dae2b6275e4d34c125e5ee51e394b7d3a18880",
            "0x98bcc0293806b124b557f0826a7acceac7e62f27677e1678095517859e58bfbd",
            "0x25a9449724c67d31fce7a5239ab11b426e00eb77410e7b3037255f1b9a1310d3",
            "0x32d5fb4fa3b1ea97d885ce8b8a5f3a93b1c805fc3229ecb2c68e9af28610df8",
            "0x937b2082d0eb3a48980c3b54501aa0fbad83f4ba483f0db0d066dc22d2696e95",
            "0xd264218db2a9722580462c97c8c0b713f45e629164b63f9b606fa1f394fbd25f",
            "0xcd31547d57cb9ccd04d1bfb1af4cb34be177a69fb950eccd7ca4e98e95234224",
            "0x68562d0a4a43e1977442b4040238c2c9940ffb418f852ee02d23d776b3a79693",
            "0x32502c66fc24b4b4df2fca84c6cd5263e88d501bc1cc1ae5839cec2397911b62",
            "0x6fd903106aeadd4ad9f92c0c0f07c7565a7cd8d8dce9f4e35f01d8d0453b3157",
            "0x380dcffa86348ab8e14b6d77263e047cc5c38f27f06dff220bffd2d5d0a32d99",
            "0x39191a3a732fcb00da927ecb5b53cbfe6c2d2bebf7a65acfead1442411989e77",
            "0x380dcffa86348ab8e14b6d77263e047cc5c38f27f06dff220bffd2d5d0a32d99",
            "0xc6e6e5c58cd034ff256d8134a4ac340193d2d4140859a530152ebbdaee675db8",
            "0x32502c66fc24b4b4df2fca84c6cd5263e88d501bc1cc1ae5839cec2397911b62",
            "0x9026fcef951522b52606d3f3f0f838a9a583272723160b1ca0fe272ebac4cad8",
            "0xcd31547d57cb9ccd04d1bfb1af4cb34be177a69fb950eccd7ca4e98e95234224",
            "0x97a9d2f5b5bc1e688bbd4bfbfdc73d366bf004be707ad11fd2dc28884c58659c",
            "0x937b2082d0eb3a48980c3b54501aa0fbad83f4ba483f0db0d066dc22d2696e95",
            "0x2d9bde724d568dda7fb9d368373f48ec0ba19d6e9b49c0649f905e0b6b0429d0",
            "0x25a9449724c67d31fce7a5239ab11b426e00eb77410e7b3037255f1b9a1310d3",
            "0xfcd2a04b05c4e1568277a317475a0c56c4e37fa03cdd6134d397164fd79eee37",
            "0x6f997fb77607bc634239dbfa14dae2b6275e4d34c125e5ee51e394b7d3a18880",
            "0x67433fd6c7f94edb4aa80f7d958533153819d0d89881e987f6aae87961a73c72",
            "0x55c63d63f7302880d113baf54f8e2e954df5a2c2b9c266c88aaedb712bb01581",
            "0xf3a8006ff73c8141713a29716ad0428abc4154242e988f657c9039e6504c89ac",
            "0xf212ceb1de39d4f60b913e04990ac959c43369da9a6bac46f0e55e4298228aff",
            "0xfea8c1c1fdfd7921fd112f54b6615ceb46185fc8493e536008c22d7f74a60d3e",
        ])?;

        assert_eq!(computed_table, table);

        Ok(())
    }

    #[test]
    fn test_precomputation_base_point() -> Result<()> {
        let precompute = KERNEL.global_labels["precompute_table_base_point"];

        let initial_stack = u256ify(["0xdeadbeef"])?;
        let mut int = Interpreter::new(
            &KERNEL.code,
            precompute,
            initial_stack,
            &KERNEL.prover_inputs,
        );
        int.run()?;

        let mut computed_table = Vec::new();
        for i in 0..32 {
            computed_table.push(int.generation_state.memory.mload_general(
                0,
                Segment::EcdsaTableG,
                i,
            ));
        }

        let table = u256ify([
            "0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            "0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8",
            "0xf9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9",
            "0x388f7b0f632de8140fe337e62a37f3566500a99934c2231b6cb9fd7584b8e672",
            "0x2f8bde4d1a07209355b4a7250a5c5128e88b84bddc619ab7cba8d569b240efe4",
            "0xd8ac222636e5e3d6d4dba9dda6c9c426f788271bab0d6840dca87d3aa6ac62d6",
            "0x5cbdf0646e5db4eaa398f365f2ea7a0e3d419b7e0330e39ce92bddedcac4f9bc",
            "0x6aebca40ba255960a3178d6d861a54dba813d0b813fde7b5a5082628087264da",
            "0xacd484e2f0c7f65309ad178a9f559abde09796974c57e714c35f110dfc27ccbe",
            "0xcc338921b0a7d9fd64380971763b61e9add888a4375f8e0f05cc262ac64f9c37",
            "0x774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb",
            "0xd984a032eb6b5e190243dd56d7b7b365372db1e2dff9d6a8301d74c9c953c61b",
            "0xf28773c2d975288bc7d1d205c3748651b075fbc6610e58cddeeddf8f19405aa8",
            "0xab0902e8d880a89758212eb65cdaf473a1a06da521fa91f29b5cb52db03ed81",
            "0xd7924d4f7d43ea965a465ae3095ff41131e5946f3c85f79e44adbcf8e27e080e",
            "0x581e2872a86c72a683842ec228cc6defea40af2bd896d3a5c504dc9ff6a26b58",
            "0xd7924d4f7d43ea965a465ae3095ff41131e5946f3c85f79e44adbcf8e27e080e",
            "0xa7e1d78d57938d597c7bd13dd733921015bf50d427692c5a3afb235f095d90d7",
            "0xf28773c2d975288bc7d1d205c3748651b075fbc6610e58cddeeddf8f19405aa8",
            "0xf54f6fd17277f5768a7ded149a3250b8c5e5f925ade056e0d64a34ac24fc0eae",
            "0x774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb",
            "0x267b5fcd1494a1e6fdbc22a928484c9ac8d24e1d20062957cfe28b3536ac3614",
            "0xacd484e2f0c7f65309ad178a9f559abde09796974c57e714c35f110dfc27ccbe",
            "0x33cc76de4f5826029bc7f68e89c49e165227775bc8a071f0fa33d9d439b05ff8",
            "0x5cbdf0646e5db4eaa398f365f2ea7a0e3d419b7e0330e39ce92bddedcac4f9bc",
            "0x951435bf45daa69f5ce8729279e5ab2457ec2f47ec02184a5af7d9d6f78d9755",
            "0x2f8bde4d1a07209355b4a7250a5c5128e88b84bddc619ab7cba8d569b240efe4",
            "0x2753ddd9c91a1c292b24562259363bd90877d8e454f297bf235782c459539959",
            "0xf9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9",
            "0xc77084f09cd217ebf01cc819d5c80ca99aff5666cb3ddce4934602897b4715bd",
            "0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            "0xb7c52588d95c3b9aa25b0403f1eef75702e84bb7597aabe663b82f6f04ef2777",
        ])?;

        assert_eq!(computed_table, table);

        Ok(())
    }

    #[test]
    fn test_glv_verify_data() -> Result<()> {
        let glv = KERNEL.global_labels["glv_decompose"];

        let f = include_str!("glv_test_data");
        for line in f.lines().filter(|s| !s.starts_with("//")) {
            let mut line = line
                .split_whitespace()
                .map(|s| U256::from_str_radix(s, 10).unwrap())
                .collect::<Vec<_>>();
            let k = line.remove(0);
            line.reverse();

            let mut initial_stack = u256ify(["0xdeadbeef"])?;
            initial_stack.push(k);
            let mut int = Interpreter::new(&KERNEL.code, glv, initial_stack, &KERNEL.prover_inputs);
            int.run()?;

            assert_eq!(line, int.stack());
        }

        Ok(())
    }
}
