use std::marker::PhantomData;
use std::time::{Duration, Instant};

use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::iop::wire::Wire;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_field::extension::Extendable;
use plonky2_field::goldilocks_field::GoldilocksField;
use plonky2_u32::gadgets::arithmetic_u32::U32Target;
use plonky2_u32::gadgets::multiple_comparison::list_le_u32_circuit;
use plonky2_u32::gates::comparison::ComparisonGate;

use crate::bit_operations::{add_arr, and_arr, not_arr, xor2_arr, xor3_arr, zip_add};
use crate::helper::byte_to_u32_target;
use crate::helper::{_right_rotate, _shr, uint32_to_bits};
use crate::sha256::make_sha256_circuit;
pub struct HeaderTarget {
    header_bits: Vec<BoolTarget>,
    threshold_bits: Vec<BoolTarget>,
    hash: Vec<BoolTarget>,
    work: Target,
}

pub fn make_header_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
) -> HeaderTarget {
    let mut header_bits = Vec::new();
    for _ in 0..80 * 8 {
        // 80 bytes in a header
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
    for i in 0..256 {
        return_hash.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
        builder.connect(sha2_targets.digest[i].target, return_hash[i].target);
    }

    println!("Double Sha finished");

    // TODO should be in a different circuit
    // Deal with the difficulty
    // Extract difficulty bits from the 80 bytes
    let mut threshold_bits_input = Vec::new();
    let mut threshold_bits = Vec::new();
    for i in 0..256 {
        threshold_bits_input.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
        threshold_bits.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
        builder.connect(threshold_bits_input[i].target, threshold_bits[i].target);
    }

    let mut difficulty_exp_bits = header_bits[600..608].to_vec();
    let mut difficulty_exp_int = byte_to_u32_target(builder, difficulty_exp_bits);

    // Check if threshold array is all 0 OR in the range of mantissa
    for j in 0..32 {
        let byte_from_bits =
            byte_to_u32_target(builder, threshold_bits[j * 8..(j + 1) * 8].to_vec()).0;

        let zero1 = builder.zero();
        let is_zero = builder.is_equal(byte_from_bits, zero1).target;

        let const_index = builder.constant(F::from_canonical_u64(j as u64));

        let const32 = builder.constant(F::from_canonical_u64(32));
        let index1 = builder.sub(const32, difficulty_exp_int.0);
        let is_first_mantissa_byte = builder.is_equal(const_index, index1);

        let index2 = builder.add_const(index1, F::ONE);
        let is_second_mantissa_byte = builder.is_equal(const_index, index2);

        let index3 = builder.add_const(index2, F::ONE);
        let is_third_mantissa_byte = builder.is_equal(const_index, index3);

        let sum1 = builder.add(
            is_first_mantissa_byte.target,
            is_second_mantissa_byte.target,
        );
        let is_in_range = builder.add(sum1, is_third_mantissa_byte.target);

        let in_range_or_equals_zero = builder.add(is_zero, is_in_range);
        let const0 = builder.constant(F::ZERO);
        let mistake_exists = builder.is_equal(in_range_or_equals_zero, const0);

        let _false = builder._false();
        builder.connect(mistake_exists.target, _false.target);
    }

    println!("Checked that all bits are in mantissa range or equals zero");

    // Check that mantissa range matches mantissa from 80-byte header
    // However, it's annoying because mantissa from the header is in little-endian by BYTES

    let mut threshold_bytes = Vec::new();
    for j in 0..32 {
        threshold_bytes.push(builder.add_virtual_target()); // Will verify that input is 0 or 1

        let byte_from_bits =
            byte_to_u32_target(builder, threshold_bits[j * 8..(j + 1) * 8].to_vec()).0;
        builder.connect(threshold_bytes[j], byte_from_bits);
    }

    let mut check_bytes = |threshold_byte_index: u64, header_bit_index: usize| {
        // Check left-most byte of threshold bits
        let mut new_threshold_bytes = Vec::new();
        for j in 0..32 {
            new_threshold_bytes.push(builder.add_virtual_target());
            builder.connect(new_threshold_bytes[j], threshold_bytes[j]);
        }

        let thirty_two = builder.constant(F::from_canonical_u64(threshold_byte_index));
        let mut access_index = builder.sub(thirty_two, difficulty_exp_int.0);

        let threshold_byte = builder.random_access(access_index, new_threshold_bytes);

        // Check that threshold_bits matches mantissa
        let header_byte = byte_to_u32_target(
            builder,
            header_bits[header_bit_index..header_bit_index + 8].to_vec(),
        )
        .0;
        builder.connect(threshold_byte, header_byte);
    };

    check_bytes(32, 592);
    check_bytes(33, 584);
    check_bytes(34, 576);

    println!("Bytes comparison done");

    let mut sha2_bytes = Vec::new();
    for j in 0..32 {
        sha2_bytes.push(builder.add_virtual_target()); // Will verify that input is 0 or 1

        let byte_from_bits =
            byte_to_u32_target(builder, sha1_targets.digest[j * 8..(j + 1) * 8].to_vec()).0;
        builder.connect(sha2_bytes[j], byte_from_bits);
    }

    println!("here2");

    // Compare difficulty_bits with output of double SHA 256
    let is_less = list_le_u32_circuit(
        builder,
        threshold_bytes.into_iter().map(|x| U32Target(x)).collect(),
        sha2_bytes.into_iter().map(|x| U32Target(x)).collect(),
    );

    println!("here3");

    let one = builder._true();
    builder.connect(is_less.target, one.target);

    return HeaderTarget {
        header_bits: header_bits,
        threshold_bits: threshold_bits_input,
        hash: return_hash,
        work: builder.constant(F::ONE), // TODO
    };
}

pub struct MultiHeaderTarget {
    pub headers: Vec<BoolTarget>,
    pub multi_threshold_bits: Vec<BoolTarget>,
    pub total_work: Target,
    pub hashes: Vec<Vec<BoolTarget>>,
}

pub fn make_multi_header_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    num_headers: usize,
) -> MultiHeaderTarget {
    let mut multi_header_bits = Vec::new();
    for _ in 0..num_headers * 80 * 8 {
        // 80 bytes in a header, each byte is 8 bits
        multi_header_bits.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
    }

    let mut multi_threshold_bits = Vec::new();
    for _ in 0..num_headers * 256 {
        // 256 bits in a header
        multi_threshold_bits.push(builder.add_virtual_bool_target_safe()); // Will verify that input is 0 or 1
    }

    let mut hashes = Vec::new();
    let mut work = Vec::new();

    for h in 0..num_headers {
        // First make the header work verification circuit and pass in the relevant header
        let header_targets = make_header_circuit(builder);
        for i in 0..80 * 8 {
            builder.connect(
                header_targets.header_bits[i].target,
                multi_header_bits[(h * 8 * 80) + i].target,
            );
        }

        for i in 0..256 {
            builder.connect(
                header_targets.threshold_bits[i].target,
                multi_threshold_bits[h * 256 + i].target,
            );
        }

        println!("Header {}", h);

        // Then add the header's work to the total work
        if h == 0 {
            work.push(header_targets.work);
        } else {
            work.push(builder.add(work[h - 1], header_targets.work));
        }

        hashes.push(header_targets.hash);

        if h > 0 {
            // Make sure that the header connects to the previous header's hash
            let claimed_prev_header =
                &multi_header_bits[(h * 80 * 8) + 4 * 8..(h * 80 * 8) + 36 * 8];
            for i in 0..256 {
                builder.connect(hashes[h - 1][i].target, claimed_prev_header[i].target);
            }
        }
    }

    return MultiHeaderTarget {
        headers: multi_header_bits,
        multi_threshold_bits: multi_threshold_bits,
        total_work: work[num_headers - 1],
        hashes: hashes,
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

    fn compute_exp_and_mantissa(header_bits: Vec<bool>) -> (u32, u64) {
        let mut d = 0;
        for i in 600..608 {
            d += ((header_bits[i]) as u32) << (608 - i - 1);
        }
        let exp = 8 * (d - 3);
        let mut mantissa = 0;
        for i in 576..584 {
            mantissa += ((header_bits[i]) as u64) << (584 - i - 1);
        }
        for i in 584..592 {
            mantissa += ((header_bits[i]) as u64) << (592 - i - 1 + 8);
        }
        for i in 592..600 {
            mantissa += ((header_bits[i]) as u64) << (600 - i - 1 + 16);
        }

        (exp, mantissa)
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

        println!("{}", header_bits.len());

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

        let (exp, mantissa) = compute_exp_and_mantissa(header_bits);

        println!("exp: {}, mantissa: {}", exp, mantissa);

        for i in 0..256 {
            if i < 256 - exp && mantissa & (1 << (255 - exp - i)) != 0 {
                pw.set_bool_target(targets.threshold_bits[i as usize], true);
                print!("1");
            } else {
                pw.set_bool_target(targets.threshold_bits[i as usize], false);
                print!("0");
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
            "0508085c47cc849eb80ea905cc7800a3be674ffc57263cf210c59d8d00000000",
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
                    builder.assert_one(targets.hashes[num_headers - 1][i].target);
                } else {
                    builder.assert_zero(targets.hashes[num_headers - 1][i].target);
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

            let (exp, mantissa) = compute_exp_and_mantissa(header_bits);

            for i in 0..256 {
                if i < 256 - exp && mantissa & (1 << (255 - exp - i)) != 0 {
                    pw.set_bool_target(targets.multi_threshold_bits[h * 256 + i as usize], true);
                    print!("1");
                } else {
                    pw.set_bool_target(targets.multi_threshold_bits[h * 256 + i as usize], false);
                    print!("0");
                }
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

use plonky2::gates::gate::Gate;
use plonky2::gates::packed_util::PackedEvaluableBase;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::witness::{PartitionWitness, Witness};
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use plonky2_field::packed::PackedField;
use plonky2_field::types::Field;

/// A gate which can perform a weighted multiply-add, i.e. `result = c0 x y + c1 z`. If the config
/// supports enough routed wires, it can support several such operations in one gate.
#[derive(Debug, Clone)]
pub struct XOR3Gate {
    pub num_xors: usize,
}

impl XOR3Gate {
    pub fn new(num_xors: usize) -> Self {
        Self { num_xors }
    }

    pub fn wire_ith_a(i: usize) -> usize {
        i * 4
    }

    pub fn wire_ith_b(i: usize) -> usize {
        i * 4 + 1
    }

    pub fn wire_ith_c(i: usize) -> usize {
        i * 4 + 2
    }

    pub fn wire_ith_d(i: usize) -> usize {
        i * 4 + 3
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for XOR3Gate {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::new();

        let one = F::Extension::from_canonical_u64(1);
        let two = F::Extension::from_canonical_u64(2);
        let four = F::Extension::from_canonical_u64(4);
        let mut acc = F::Extension::from_canonical_u64(0);

        for i in 0..self.num_xors {
            let a = vars.local_wires[XOR3Gate::wire_ith_a(i)];
            let b = vars.local_wires[XOR3Gate::wire_ith_b(i)];
            let c = vars.local_wires[XOR3Gate::wire_ith_c(i)];
            let d = vars.local_wires[XOR3Gate::wire_ith_d(i)];
            let output = a * (one - two * b - two * c + four * b * c) + b + c - two * b * c - d;
            acc += output;
        }

        constraints.push(acc);

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        let one = F::from_canonical_u64(1);
        let two = F::from_canonical_u64(2);
        let four = F::from_canonical_u64(4);

        let mut acc = F::from_canonical_u64(0);
        for i in 0..self.num_xors {
            let a = vars.local_wires[XOR3Gate::wire_ith_a(i)];
            let b = vars.local_wires[XOR3Gate::wire_ith_b(i)];
            let c = vars.local_wires[XOR3Gate::wire_ith_c(i)];
            let d = vars.local_wires[XOR3Gate::wire_ith_d(i)];
            let output = a * (one - two * b - two * c + four * b * c) + b + c - two * b * c - d;
            acc += output;
        }

        yield_constr.one(acc);
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::new();

        // let one = builder.constant_extension(F::Extension::from_canonical_u64(1));
        // let two = builder.constant_extension(F::Extension::from_canonical_u64(2));
        // let four = builder.constant_extension(F::Extension::from_canonical_u64(4));

        // let a = vars.local_wires[0];
        // let b = vars.local_wires[1];
        // let c = vars.local_wires[2];
        // let output = vars.local_wires[3];

        // let m = builder.mul_extension(b, c);
        // let two_b = builder.mul_extension(two, b);
        // let two_c = builder.mul_extension(two, c);
        // let two_m = builder.mul_extension(four, m);
        // let four_m = builder.mul_extension(four, m);

        // let result = builder.sub_extension(one, two_b);
        // let result = builder.sub_extension(result, two_c);
        // let result = builder.add_extension(result, four_m);
        // let result = builder.mul_extension(result, a);

        // let result = builder.add_extension(result, b);
        // let result = builder.add_extension(result, c);
        // let result = builder.sub_extension(result, two_m);

        // let result = builder.sub_extension(result, result);

        // constraints.push(result);

        constraints
    }

    fn generators(&self, row: usize, local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = XOR3Generator::<F, D> {
            row,
            num_xors: self.num_xors,
            _phantom: PhantomData,
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        4
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        1
    }
}

#[derive(Debug)]
struct XOR3Generator<F: RichField + Extendable<D>, const D: usize> {
    row: usize,
    num_xors: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F> for XOR3Generator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |column| Target::wire(self.row, column);
        let mut result: Vec<Target> = Vec::new();

        for i in 0..self.num_xors {
            result.push(local_target(i * 4));
            result.push(local_target(i * 4 + 1));
            result.push(local_target(i * 4 + 2));
        }

        result
    }

    /*
    a ^ b ^ c = a+b+c - 2*a*b - 2*a*c - 2*b*c + 4*a*b*c
            = a*( 1 - 2*b - 2*c + 4*b*c ) + b + c - 2*b*c
            = a*( 1 - 2*b -2*c + 4*m ) + b + c - 2*m
    where m = b*c
    */
    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let get_wire = |wire: usize| -> F { witness.get_target(Target::wire(self.row, wire)) };

        let one = F::from_canonical_u64(1);
        let two = F::from_canonical_u64(2);
        let four = F::from_canonical_u64(4);

        for i in 0..self.num_xors {
            let a = get_wire(4 * i);
            let b = get_wire(4 * i + 1);
            let c = get_wire(4 * i + 2);
            let d_target = Target::wire(self.row, 4 * i + 3);
            let computed_output =
                a * (one - two * b - two * c + four * b * c) + b + c - two * b * c;
            out_buffer.set_target(d_target, computed_output);
        }
    }
}
