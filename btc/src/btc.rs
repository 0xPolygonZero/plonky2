use crate::bit_operations::{add_arr, and_arr, not_arr, xor2_arr, xor3_arr, zip_add};
use crate::helper::{_right_rotate, _shr, uint32_to_bits};
use crate::sha256::make_sha256_circuit;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_field::extension::Extendable;
use plonky2_field::goldilocks_field::GoldilocksField;
use plonky2_u32::gadgets::multiple_comparison::list_le_circuit;
use plonky2_u32::gates::comparison::ComparisonGate;
use plonky2::gates::random_access::RandomAccessGate;
use std::time::{Duration, Instant};
use plonky2_ecdsa::gadgets::biguint::{CircuitBuilderBiguint, BigUintTarget};

use super::helper::bits_to_biguint_target;
pub struct HeaderTarget {
    header_bits: Vec<BoolTarget>,
    threshold_bits: Vec<BoolTarget>,
    hash: Vec<BoolTarget>,
    work: BigUintTarget
}

pub fn make_header_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>
) -> HeaderTarget 
{
    let mut header_bits = Vec::new();
    for _ in 0..80 * 8 { // 80 bytes in a header
        header_bits.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
    }

    let sha1_targets = make_sha256_circuit(builder, header_bits.len() as u128);

    for i in 0..header_bits.len() {
        builder.connect(header_bits[i].target, sha1_targets.message[i].target);
    }

    let sha2_targets = make_sha256_circuit(builder, sha1_targets.digest.len() as u128);

    for i in 0..sha1_targets.digest.len() {
        builder.connect(
            sha1_targets.digest[i].target,
            sha2_targets.message[i].target,
        );
    }


    let mut return_hash = Vec::new();
    for i in 0..256{ // 80 bytes in a header
        return_hash.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
        builder.connect(sha2_targets.digest[i].target, return_hash[i].target);
    }

    // TODO should be in a different circuit
    // Deal with the difficulty
    // Extract difficulty bits from the 80 bytes
    let mut threshold_bits_input = Vec::new();
    let mut threshold_bits = Vec::new();
    for i in 0..80 * 8 {
        // 80 bytes in a header
        threshold_bits_input.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
        threshold_bits.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
        builder.connect(threshold_bits_input[i].target, threshold_bits[i].target);
    }


    let mut difficulty_exp_bits = header_bits[512..528].to_vec();
    let mut padded_difficulty_bits = Vec::new();
    for i in 0..32 {
        padded_difficulty_bits.push(builder.add_virtual_bool_target_safe());
        if (i > 15  ) {
            let zero = builder.zero();
            builder.connect(padded_difficulty_bits[i].target, zero);
        } else {
            builder.connect(difficulty_exp_bits[i].target, padded_difficulty_bits[i].target);
        }
    }
    let mut difficulty_exp_int = bits_to_biguint_target(builder, padded_difficulty_bits).limbs[0];

    let inter1 = builder.mul_const(
        F::from_canonical_u64(8),
        difficulty_exp_int.0,
    );
    let const1 =  builder.constant(F::from_canonical_u64(232));
    let mut mantissa_start_index = builder.sub(
        const1,
        inter1
    );

    // Check if threshold array is all 0 OR in the range of mantissa
    for i in 0..256 {
        let a_le_b_gate = ComparisonGate::new(9, 1);
        let a_le_b_row = builder.add_gate(a_le_b_gate.clone(), vec![]);
        builder.connect(
            Target::wire(a_le_b_row, a_le_b_gate.wire_first_input()),
            mantissa_start_index,
        );
        builder.connect(
            Target::wire(a_le_b_row, a_le_b_gate.wire_second_input()),
            threshold_bits[i].target,
        );
        let first_result = Target::wire(a_le_b_row, a_le_b_gate.wire_result_bool());

        let a_le_b_gate = ComparisonGate::new(9, 1);
        let a_le_b_row = builder.add_gate(a_le_b_gate.clone(), vec![]);
        builder.connect(
            Target::wire(a_le_b_row, a_le_b_gate.wire_first_input()),
            threshold_bits[i].target,
        );
        let inter2 =builder.add_const(
            mantissa_start_index,
            F::from_canonical_u64(47),
        );
        builder.connect(
            Target::wire(a_le_b_row, a_le_b_gate.wire_second_input()),
            inter2
        );
        let second_result = Target::wire(a_le_b_row, a_le_b_gate.wire_result_bool());

        {
            let zero1 = builder.zero();
            let const1 = builder.is_equal(threshold_bits[i].target, zero1).target;
            let const2 = builder.and(BoolTarget::new_unsafe(first_result), BoolTarget::new_unsafe(second_result)).target;
            let const3 = builder.add(const1, const2);
            let zero2 = builder.zero();
            let const4 = builder.is_equal(const3, zero2);

            let in_range_or_equals_zero = builder.not(const4);
            let true1 = builder._true();
            builder.connect(in_range_or_equals_zero.target, true1.target);
        }

    }

    // Check that mantissa range matches mantissa from 80 bytes
    let claimed_element = builder.add_virtual_target();

    let random_gate = RandomAccessGate::<F, D>::new_from_config(&builder.config, 256);
    let (row, copy) = builder.find_slot(random_gate, &[], &[]);

    threshold_bits.iter().enumerate().for_each(|(i, &val)| {
        builder.connect(
            val.target,
            Target::wire(row, random_gate.wire_list_item(i, copy)),
        );
    });

    for i in 0..48 {
        {
            let const1 = builder.constant(F::from_canonical_u64(232 + i));
            let mul1 = builder.mul_const(
                F::from_canonical_u64(8),
                difficulty_exp_int.0,
            );
            let mut access_index = builder.sub(
                const1,
                mul1
            );
            builder.connect(
                access_index,
                Target::wire(row, random_gate.wire_access_index(copy)),
            );
        }
        
        builder.connect(
            claimed_element,
            Target::wire(row, random_gate.wire_claimed_element(copy)),
        );

        // Check that threshold_bits matches mantissa
        builder.connect(claimed_element, header_bits[(528 + i) as usize].target);
    }

    // Compare difficulty_bits with output of double SHA 256
    let is_less = list_le_circuit(
        builder,
        threshold_bits.into_iter().map(|x| x.target).collect(),
        sha2_targets.digest.into_iter().map(|x| x.target).collect(),
        256,
    );

    let one = builder._true();
    builder.connect(is_less.target, one.target);

    // let mut difficulty_bits = Vec::new();
    // for i in 0..256 {
    //     let mut agg = builder.constant_bool(true);
    //     let mut chosen_bit = builder.constant_bool(false);
    //     for j in 0..48 {
    //         let mut check = builder.is_equal(
    //             builder.constant(F::from_canonical_u64(i)),
    //             builder.add(
    //                 builder.constant(F::from_canonical_u64(j)),
    //                 mantissa_start_index,
    //             ),
    //         );
    //         agg = builder.and(agg, builder.not(check));
    //         chosen_bit = builder.select(
    //             check,
    //             header_bits[(528 + j) as usize].target,
    //             chosen_bit.target,
    //         );
    //         // chosen_bit = builder.select(check, difficulty_mantissa_bits[j].target, chosen_bit.target);
    //     }
    //     difficulty_bits.push(builder.select(builder.not(agg), chosen_bit, F::ZERO));
    // }

    // Now we compute the work given the threshold bits
    let mut numerator_bits = Vec::new(); // 2^256
    let mut threshold_bits_copy = Vec::new();
    for i in 0..256 {
        if i == 0 {
            numerator_bits.push(builder.constant_bool(true));
        } else {
            numerator_bits.push(builder.constant_bool(false));
        }
        threshold_bits_copy.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
        builder.connect(threshold_bits_input[i].target, threshold_bits_copy[i].target);
    }
    let numerator_as_biguint = bits_to_biguint_target(builder, numerator_bits);
    let denominator = bits_to_biguint_target(builder, threshold_bits_copy);
    let work = builder.div_biguint(&numerator_as_biguint, &denominator);

    return HeaderTarget {
        header_bits: header_bits,
        threshold_bits: threshold_bits_input,
        hash: return_hash,
        work: work
    };
}

pub struct MultiHeaderTarget {
    pub headers: Vec<BoolTarget>,
    pub total_work: BigUintTarget,
    pub hashes: Vec<Vec<BoolTarget>>,
}

pub fn make_multi_header_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    num_headers: usize,
) -> MultiHeaderTarget 
{
    let mut multi_header_bits = Vec::new();
    for _ in 0..num_headers * 80 * 8 { // 80 bytes in a header, each byte is 8 bits
        multi_header_bits.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
    }

    let mut hashes = Vec::new();
    let mut work = Vec::new();

    for h in 0 .. num_headers {
        // First make the header work verification circuit and pass in the relevant header
        let header_targets = make_header_circuit(builder);
        for i in 0..80 * 8 {
            builder.connect(header_targets.header_bits[i].target, multi_header_bits[(h*8*80) + i].target);
        }

        println!("Header {}", h);
    
        // Then add the header's work to the total work
        if h == 0 {
            work.push(header_targets.work);
        } else {
            work.push(builder.add_biguint(&work[h-1], &header_targets.work));
        }

        hashes.push(header_targets.hash);

        if h > 0 {
            // Make sure that the header connects to the previous header's hash
            let claimed_prev_header = &multi_header_bits[(h * 80 * 8) + 4 * 8.. (h * 80 * 8) + 36 * 8];
            for i in 0..256 {
                builder.connect(hashes[h-1][i].target, claimed_prev_header[i].target);
            }
        }
    }

    let total_work = builder.add_virtual_biguint_target(work[0].num_limbs());
    builder.connect_biguint(&work[work.len() - 1], &total_work);

    return MultiHeaderTarget {
        headers: multi_header_bits,
        total_work: total_work,
        hashes: hashes
    };
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use hex::decode;
    use plonky2::iop::witness::{PartialWitness, Witness};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::btc::make_header_circuit;
    use crate::btc::make_multi_header_circuit;

    fn to_bits(msg: Vec<u8>) -> Vec<bool> {
        let mut res = Vec::new();
        for i in 0..msg.len() {
            let char = msg[i];
            for j in 0..8 {
                if (char & (1 << 7 - j)) != 0 {
                    res.push(true);
                } else {
                    res.push(false);
                }
            }
        }
        res
    }

    #[test]
    fn test_header_circuit() -> Result<()> {
        let genesis_header = decode("0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c").unwrap();
        let header_bits = to_bits(genesis_header);
        // NOTE this is the reversed order of how it's displayed on block explorers
        let expected_hash = "6fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000";
        let hash_bits = to_bits(decode(expected_hash).unwrap());

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let targets = make_header_circuit(&mut builder);
       
        for i in 0..hash_bits.len() {
            if hash_bits[i] {
                builder.assert_one(targets.hash[i].target);
            } else {
                builder.assert_zero(targets.hash[i].target);
            }
        }

        let data = builder.build::<C>();

        let mut pw = PartialWitness::new();
        for i in 0..header_bits.len() {
            pw.set_bool_target(targets.header_bits[i], header_bits[i]);
        }
        let mut exp = 0;
        for i in 512..512 + 16 {
            exp += ((header_bits[i]) as u32) << (i - 512);
        }
        let mut mantissa = 0;
        for i in 512 + 16 .. 512+64 {
            mantissa += ((header_bits[i]) as u64) << (i - (512 + 16));
        }

        let threshold = mantissa * 2u64.pow(8 * (exp - 3));
        println!("Threshold: {}", threshold);

        for i in 0..256 {
            if threshold & (1 << (255 - i)) != 0 {
                pw.set_bool_target(targets.threshold_bits[i], true);
            } else {
                pw.set_bool_target(targets.threshold_bits[i], false);
            }
        }

        let now = std::time::Instant::now();
        let proof = data.prove(pw).unwrap();
        let elapsed = now.elapsed().as_millis();
        println!("Proved the circuit in {} ms", elapsed);
        data.verify(proof)

    }

    #[test]
    fn test_multi_header_circuit() -> Result<()> {
        let num_headers = 4;
        let headers = [
            "0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c",
            "010000006fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000982051fd1e4ba744bbbe680e1fee14677ba1a3c3540bf7b1cdb606e857233e0e61bc6649ffff001d01e36299",
            "010000004860eb18bf1b1620e37e9490fc8a427514416fd75159ab86688e9a8300000000d5fdcc541e25de1c7a5addedf24858b8bb665c9f36ef744ee42c316022c90f9bb0bc6649ffff001d08d2bd61",
            "01000000bddd99ccfda39da1b108ce1a5d70038d0a967bacb68b6b63065f626a0000000044f672226090d85db9a9f2fbfe5f0f9609b387af7be5b7fbb7a1767c831c9e995dbe6649ffff001d05e0ed6d",
            "010000004944469562ae1c2c74d9a535e00b6f3e40ffbad4f2fda3895501b582000000007a06ea98cd40ba2e3288262b28638cec5337c1456aaf5eedc8e9e5a20f062bdf8cc16649ffff001d2bfee0a9",
            "0100000085144a84488ea88d221c8bd6c059da090e88f8a2c99690ee55dbba4e00000000e11c48fecdd9e72510ca84f023370c9a38bf91ac5cae88019bee94d24528526344c36649ffff001d1d03e477",
            "01000000fc33f596f822a0a1951ffdbf2a897b095636ad871707bf5d3162729b00000000379dfb96a5ea8c81700ea4ac6b97ae9a9312b2d4301a29580e924ee6761a2520adc46649ffff001d189c4c97",
            "010000008d778fdc15a2d3fb76b7122a3b5582bea4f21f5a0c693537e7a03130000000003f674005103b42f984169c7d008370967e91920a6a5d64fd51282f75bc73a68af1c66649ffff001d39a59c86",
            "010000004494c8cf4154bdcc0720cd4a59d9c9b285e4b146d45f061d2b6c967100000000e3855ed886605b6d4a99d5fa2ef2e9b0b164e63df3c4136bebf2d0dac0f1f7a667c86649ffff001d1c4b5666",
            "01000000c60ddef1b7618ca2348a46e868afc26e3efc68226c78aa47f8488c4000000000c997a5e56e104102fa209c6a852dd90660a20b2d9c352423edce25857fcd37047fca6649ffff001d28404f53"
        ];
        let expected_hashes = [
            "6fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000",
            "4860eb18bf1b1620e37e9490fc8a427514416fd75159ab86688e9a8300000000",
            "bddd99ccfda39da1b108ce1a5d70038d0a967bacb68b6b63065f626a00000000",
            "4944469562ae1c2c74d9a535e00b6f3e40ffbad4f2fda3895501b58200000000",
            "85144a84488ea88d221c8bd6c059da090e88f8a2c99690ee55dbba4e00000000",
            "fc33f596f822a0a1951ffdbf2a897b095636ad871707bf5d3162729b00000000",
            "8d778fdc15a2d3fb76b7122a3b5582bea4f21f5a0c693537e7a0313000000000",
            "4494c8cf4154bdcc0720cd4a59d9c9b285e4b146d45f061d2b6c967100000000",
            "c60ddef1b7618ca2348a46e868afc26e3efc68226c78aa47f8488c4000000000",
            "0508085c47cc849eb80ea905cc7800a3be674ffc57263cf210c59d8d00000000"
        ];

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let targets = make_multi_header_circuit(&mut builder, num_headers);

        {
            let hash_bits = to_bits(decode(expected_hashes[0]).unwrap());
            for i in 0..256 {
                if hash_bits[i] {
                    builder.assert_one(targets.hashes[0][i].target);
                } else {
                    builder.assert_zero(targets.hashes[0][i].target);
                }
            }
        } 
        {
            let hash_bits = to_bits(decode(expected_hashes[num_headers - 1]).unwrap());
            for i in 0..256 {
                if hash_bits[i] {
                    builder.assert_one(targets.hashes[num_headers-1][i].target);
                } else {
                    builder.assert_zero(targets.hashes[num_headers-1][i].target);
                }
            }
        }

        let data = builder.build::<C>();
        println!("Built the circuit");

        let mut pw = PartialWitness::new();
        for h in 0..num_headers {
            let header_bits = to_bits(decode(headers[h]).unwrap());
            for i in 0..80 * 8 {
                pw.set_bool_target(targets.headers[h * 80 * 8 + i], header_bits[i]);
            }
        }

        let now = std::time::Instant::now();
        let proof = data.prove(pw).unwrap();
        let elapsed = now.elapsed().as_millis();
        println!("Proved the circuit in {} ms", elapsed);

        data.verify(proof)
    }

    #[test]
    #[should_panic]
    fn test_header_failure() {
        let mut header = String::new();
        for _ in 0..8 {
            header.push_str("abcdefghij");
        }
        let header_bits = to_bits(header.as_bytes().to_vec());
        let expected_hash = "d68d62c262c2ec08961c1104188cde86f51695878759666ad61490c8ec66745c";
        let hash_bits = to_bits(decode(expected_hash).unwrap());

        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut builder = CircuitBuilder::<F, D>::new(CircuitConfig::standard_recursion_config());
        let targets = make_header_circuit(&mut builder);
       
        for i in 0..hash_bits.len() {
            if hash_bits[i] {
                builder.assert_one(targets.hash[i].target);
            } else {
                builder.assert_zero(targets.hash[i].target);
            }
        }

        let data = builder.build::<C>(); 
        let mut pw = PartialWitness::new();

        for i in 0..header_bits.len() {
            pw.set_bool_target(targets.header_bits[i], header_bits[i]);
        }
        let now = std::time::Instant::now();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).expect("header failure");
    }    

}
