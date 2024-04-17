use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::Result;
use ethereum_types::U256;
use itertools::Itertools;
use num::{BigUint, One, Zero};
use num_bigint::RandBigInt;
use plonky2_util::ceil_div_usize;
use rand::Rng;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::util::{biguint_to_mem_vec, mem_vec_to_biguint};

const BIGNUM_LIMB_BITS: usize = 128;
const MINUS_ONE: U256 = U256::MAX;

const TEST_DATA_BIGNUM_INPUTS: &str = "bignum_inputs";
const TEST_DATA_U128_INPUTS: &str = "u128_inputs";

const TEST_DATA_SHR_OUTPUTS: &str = "shr_outputs";
const TEST_DATA_ISZERO_OUTPUTS: &str = "iszero_outputs";
const TEST_DATA_CMP_OUTPUTS: &str = "cmp_outputs";
const TEST_DATA_ADD_OUTPUTS: &str = "add_outputs";
const TEST_DATA_ADDMUL_OUTPUTS: &str = "addmul_outputs";
const TEST_DATA_MUL_OUTPUTS: &str = "mul_outputs";
const TEST_DATA_MODMUL_OUTPUTS: &str = "modmul_outputs";
const TEST_DATA_MODEXP_OUTPUTS: &str = "modexp_outputs";
const TEST_DATA_MODEXP_OUTPUTS_FULL: &str = "modexp_outputs_full";

const BIT_SIZES_TO_TEST: [usize; 15] = [
    0, 1, 2, 127, 128, 129, 255, 256, 257, 512, 1000, 1023, 1024, 1025, 31415,
];

fn full_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/cpu/kernel/tests/bignum/test_data");
    path.push(filename);
    path
}

fn test_data_biguint(filename: &str) -> Vec<BigUint> {
    let file = File::open(full_path(filename)).unwrap();
    let lines = BufReader::new(file).lines();
    lines
        .map(|line| BigUint::parse_bytes(line.unwrap().as_bytes(), 10).unwrap())
        .collect()
}

fn test_data_u128(filename: &str) -> Vec<u128> {
    let file = File::open(full_path(filename)).unwrap();
    let lines = BufReader::new(file).lines();
    lines
        .map(|line| line.unwrap().parse::<u128>().unwrap())
        .collect()
}

fn test_data_u256(filename: &str) -> Vec<U256> {
    let file = File::open(full_path(filename)).unwrap();
    let lines = BufReader::new(file).lines();
    lines
        .map(|line| U256::from_dec_str(&line.unwrap()).unwrap())
        .collect()
}

// Convert each biguint to a vector of bignum limbs, pad to the given length, and concatenate.
fn pad_bignums(biguints: &[BigUint], length: usize) -> Vec<U256> {
    biguints
        .iter()
        .flat_map(|biguint| {
            biguint_to_mem_vec(biguint.clone())
                .into_iter()
                .pad_using(length, |_| U256::zero())
        })
        .collect()
}

fn gen_bignum(bit_size: usize) -> BigUint {
    let mut rng = rand::thread_rng();
    rng.gen_biguint(bit_size as u64)
}

fn max_bignum(bit_size: usize) -> BigUint {
    (BigUint::one() << bit_size) - BigUint::one()
}

fn bignum_len(a: &BigUint) -> usize {
    ceil_div_usize(a.bits() as usize, BIGNUM_LIMB_BITS)
}

fn run_test(fn_label: &str, memory: Vec<U256>, stack: Vec<U256>) -> Result<(Vec<U256>, Vec<U256>)> {
    let fn_label = KERNEL.global_labels[fn_label];
    let retdest = 0xDEADBEEFu32.into();

    let mut initial_stack: Vec<U256> = stack;
    initial_stack.push(retdest);
    initial_stack.reverse();

    let mut interpreter = Interpreter::new_with_kernel(fn_label, initial_stack);
    interpreter.set_current_general_memory(memory);
    interpreter.run()?;

    let new_memory = interpreter.get_current_general_memory();

    Ok((new_memory, interpreter.stack().to_vec()))
}

fn test_shr_bignum(input: BigUint, expected_output: BigUint) -> Result<()> {
    let len = bignum_len(&input);
    let memory = biguint_to_mem_vec(input);

    let input_start_loc = 0;
    let (new_memory, _new_stack) = run_test(
        "shr_bignum",
        memory,
        vec![len.into(), input_start_loc.into()],
    )?;

    let output = mem_vec_to_biguint(&new_memory[input_start_loc..input_start_loc + len]);
    assert_eq!(output, expected_output);

    Ok(())
}

fn test_iszero_bignum(input: BigUint, expected_output: U256) -> Result<()> {
    let len = bignum_len(&input);
    let memory = biguint_to_mem_vec(input);

    let input_start_loc = 0;
    let (_new_memory, new_stack) = run_test(
        "iszero_bignum",
        memory,
        vec![len.into(), input_start_loc.into()],
    )?;

    let output = new_stack[0];
    assert_eq!(output, expected_output);

    Ok(())
}

fn test_cmp_bignum(a: BigUint, b: BigUint, expected_output: U256) -> Result<()> {
    let len = bignum_len(&a).max(bignum_len(&b));
    let memory = pad_bignums(&[a, b], len);

    let a_start_loc = 0;
    let b_start_loc = len;
    let (_new_memory, new_stack) = run_test(
        "cmp_bignum",
        memory,
        vec![len.into(), a_start_loc.into(), b_start_loc.into()],
    )?;

    let output = new_stack[0];
    assert_eq!(output, expected_output);

    Ok(())
}

fn test_add_bignum(a: BigUint, b: BigUint, expected_output: BigUint) -> Result<()> {
    let len = bignum_len(&a).max(bignum_len(&b));
    let memory = pad_bignums(&[a, b], len);

    let a_start_loc = 0;
    let b_start_loc = len;
    let (mut new_memory, new_stack) = run_test(
        "add_bignum",
        memory,
        vec![len.into(), a_start_loc.into(), b_start_loc.into()],
    )?;

    // Determine actual sum, appending the final carry if nonzero.
    let carry_limb = new_stack[0];
    if carry_limb > 0.into() {
        new_memory[len] = carry_limb;
    }

    let expected_output = biguint_to_mem_vec(expected_output);
    let output = &new_memory[a_start_loc..a_start_loc + expected_output.len()];
    assert_eq!(output, expected_output);

    Ok(())
}

fn test_addmul_bignum(a: BigUint, b: BigUint, c: u128, expected_output: BigUint) -> Result<()> {
    let len = bignum_len(&a).max(bignum_len(&b));
    let mut memory = pad_bignums(&[a, b], len);
    memory.splice(len..len, [0.into(); 2].iter().cloned());

    let a_start_loc = 0;
    let b_start_loc = len + 2;
    let (mut new_memory, new_stack) = run_test(
        "addmul_bignum",
        memory,
        vec![len.into(), a_start_loc.into(), b_start_loc.into(), c.into()],
    )?;

    // Determine actual sum, appending the final carry if nonzero.
    let carry_limb = new_stack[0];
    if carry_limb > 0.into() {
        new_memory[len] = carry_limb;
    }

    let expected_output = biguint_to_mem_vec(expected_output);
    let output = &new_memory[a_start_loc..a_start_loc + expected_output.len()];
    assert_eq!(output, expected_output);

    Ok(())
}

fn test_mul_bignum(a: BigUint, b: BigUint, expected_output: BigUint) -> Result<()> {
    let len = bignum_len(&a).max(bignum_len(&b));
    let output_len = len * 2;
    let memory = pad_bignums(&[a, b], len);

    let a_start_loc = 0;
    let b_start_loc = len;
    let output_start_loc = 2 * len;
    let (new_memory, _new_stack) = run_test(
        "mul_bignum",
        memory,
        vec![
            len.into(),
            a_start_loc.into(),
            b_start_loc.into(),
            output_start_loc.into(),
        ],
    )?;

    let output = mem_vec_to_biguint(&new_memory[output_start_loc..output_start_loc + output_len]);
    assert_eq!(output, expected_output);

    Ok(())
}

fn test_modmul_bignum(a: BigUint, b: BigUint, m: BigUint, expected_output: BigUint) -> Result<()> {
    let len = bignum_len(&a).max(bignum_len(&b)).max(bignum_len(&m));
    let output_len = len;
    let memory = pad_bignums(&[a, b, m], len);

    let a_start_loc = 0;
    let b_start_loc = len;
    let m_start_loc = 2 * len;
    let output_start_loc = 3 * len;
    let scratch_1 = 4 * len; // size 2*len
    let scratch_2 = 6 * len; // size 2*len
    let scratch_3 = 8 * len; // size 2*len
    let (new_memory, _new_stack) = run_test(
        "modmul_bignum",
        memory,
        vec![
            len.into(),
            a_start_loc.into(),
            b_start_loc.into(),
            m_start_loc.into(),
            output_start_loc.into(),
            scratch_1.into(),
            scratch_2.into(),
            scratch_3.into(),
        ],
    )?;

    let output = mem_vec_to_biguint(&new_memory[output_start_loc..output_start_loc + output_len]);
    assert_eq!(output, expected_output);

    Ok(())
}

fn test_modexp_bignum(b: BigUint, e: BigUint, m: BigUint, expected_output: BigUint) -> Result<()> {
    let len = bignum_len(&b).max(bignum_len(&e)).max(bignum_len(&m));
    let output_len = len;
    let memory = pad_bignums(&[b, e, m], len);

    let b_start_loc = 0;
    let e_start_loc = len;
    let m_start_loc = 2 * len;
    let output_start_loc = 3 * len;
    let scratch_1 = 4 * len;
    let scratch_2 = 5 * len; // size 2*len
    let scratch_3 = 7 * len; // size 2*len
    let scratch_4 = 9 * len; // size 2*len
    let scratch_5 = 11 * len; // size 2*len
    let (mut new_memory, _new_stack) = run_test(
        "modexp_bignum",
        memory,
        vec![
            len.into(),
            b_start_loc.into(),
            e_start_loc.into(),
            m_start_loc.into(),
            output_start_loc.into(),
            scratch_1.into(),
            scratch_2.into(),
            scratch_3.into(),
            scratch_4.into(),
            scratch_5.into(),
        ],
    )?;
    new_memory.resize(
        new_memory.len().max(output_start_loc + output_len),
        0.into(),
    );

    let output = mem_vec_to_biguint(&new_memory[output_start_loc..output_start_loc + output_len]);
    assert_eq!(output, expected_output);

    Ok(())
}

#[test]
fn test_shr_bignum_all() -> Result<()> {
    for bit_size in BIT_SIZES_TO_TEST {
        let input = gen_bignum(bit_size);
        let output = input.clone() >> 1;
        test_shr_bignum(input, output)?;

        let input = max_bignum(bit_size);
        let output = input.clone() >> 1;
        test_shr_bignum(input, output)?;
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let shr_outputs = test_data_biguint(TEST_DATA_SHR_OUTPUTS);
    for (input, output) in inputs.iter().zip(shr_outputs.iter()) {
        test_shr_bignum(input.clone(), output.clone())?;
    }

    Ok(())
}

#[test]
fn test_iszero_bignum_all() -> Result<()> {
    for bit_size in BIT_SIZES_TO_TEST {
        let input = gen_bignum(bit_size);
        let output = input.is_zero() as u8;
        test_iszero_bignum(input, output.into())?;

        let input = max_bignum(bit_size);
        let output = bit_size.is_zero() as u8;
        test_iszero_bignum(input, output.into())?;
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let iszero_outputs = test_data_u256(TEST_DATA_ISZERO_OUTPUTS);
    let mut iszero_outputs_iter = iszero_outputs.iter();
    for input in inputs {
        let output = iszero_outputs_iter.next().unwrap();
        test_iszero_bignum(input.clone(), *output)?;
    }

    Ok(())
}

#[test]
fn test_cmp_bignum_all() -> Result<()> {
    for bit_size in BIT_SIZES_TO_TEST {
        let a = gen_bignum(bit_size);
        let b = gen_bignum(bit_size);
        let output = match a.cmp(&b) {
            Ordering::Less => MINUS_ONE,
            Ordering::Equal => 0.into(),
            Ordering::Greater => 1.into(),
        };
        test_cmp_bignum(a, b, output)?;

        let a = max_bignum(bit_size);
        let b = max_bignum(bit_size);
        let output = 0.into();
        test_cmp_bignum(a, b, output)?;
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let cmp_outputs = test_data_u256(TEST_DATA_CMP_OUTPUTS);
    let mut cmp_outputs_iter = cmp_outputs.iter();
    for a in &inputs {
        for b in &inputs {
            let output = cmp_outputs_iter.next().unwrap();
            test_cmp_bignum(a.clone(), b.clone(), *output)?;
        }
    }

    Ok(())
}

#[test]
fn test_add_bignum_all() -> Result<()> {
    for bit_size in BIT_SIZES_TO_TEST {
        let a = gen_bignum(bit_size);
        let b = gen_bignum(bit_size);
        let output = a.clone() + b.clone();
        test_add_bignum(a, b, output)?;

        let a = max_bignum(bit_size);
        let b = max_bignum(bit_size);
        let output = a.clone() + b.clone();
        test_add_bignum(a, b, output)?;
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let add_outputs = test_data_biguint(TEST_DATA_ADD_OUTPUTS);
    let mut add_outputs_iter = add_outputs.iter();
    for a in &inputs {
        for b in &inputs {
            let output = add_outputs_iter.next().unwrap();
            test_add_bignum(a.clone(), b.clone(), output.clone())?;
        }
    }

    Ok(())
}

#[test]
fn test_addmul_bignum_all() -> Result<()> {
    let mut rng = rand::thread_rng();

    for bit_size in BIT_SIZES_TO_TEST {
        let a = gen_bignum(bit_size);
        let b = gen_bignum(bit_size);
        let c: u128 = rng.gen();
        let output = a.clone() + b.clone() * c;
        test_addmul_bignum(a, b, c, output)?;

        let a = max_bignum(bit_size);
        let b = max_bignum(bit_size);
        let c: u128 = rng.gen();
        let output = a.clone() + b.clone() * c;
        test_addmul_bignum(a, b, c, output)?;
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let u128_inputs = test_data_u128(TEST_DATA_U128_INPUTS);
    let addmul_outputs = test_data_biguint(TEST_DATA_ADDMUL_OUTPUTS);
    let mut addmul_outputs_iter = addmul_outputs.iter();
    for a in &inputs {
        for b in &inputs {
            for c in &u128_inputs {
                let output = addmul_outputs_iter.next().unwrap();
                test_addmul_bignum(a.clone(), b.clone(), *c, output.clone())?;
            }
        }
    }

    Ok(())
}

#[test]
fn test_mul_bignum_all() -> Result<()> {
    for bit_size in BIT_SIZES_TO_TEST {
        let a = gen_bignum(bit_size);
        let b = gen_bignum(bit_size);
        let output = a.clone() * b.clone();
        test_mul_bignum(a, b, output)?;

        let a = max_bignum(bit_size);
        let b = max_bignum(bit_size);
        let output = a.clone() * b.clone();
        test_mul_bignum(a, b, output)?;
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let mul_outputs = test_data_biguint(TEST_DATA_MUL_OUTPUTS);
    let mut mul_outputs_iter = mul_outputs.iter();
    for a in &inputs {
        for b in &inputs {
            let output = mul_outputs_iter.next().unwrap();
            test_mul_bignum(a.clone(), b.clone(), output.clone())?;
        }
    }

    Ok(())
}

#[test]
fn test_modmul_bignum_all() -> Result<()> {
    for bit_size in BIT_SIZES_TO_TEST {
        let a = gen_bignum(bit_size);
        let b = gen_bignum(bit_size);
        let m = gen_bignum(bit_size);
        if !m.is_zero() {
            let output = &a * &b % &m;
            test_modmul_bignum(a, b, m, output)?;
        }

        let a = max_bignum(bit_size);
        let b = max_bignum(bit_size);
        let m = max_bignum(bit_size);
        if !m.is_zero() {
            let output = &a * &b % &m;
            test_modmul_bignum(a, b, m, output)?;
        }
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let modmul_outputs = test_data_biguint(TEST_DATA_MODMUL_OUTPUTS);
    let mut modmul_outputs_iter = modmul_outputs.into_iter();
    for a in &inputs {
        for b in &inputs {
            // For m, skip the first input, which is zero.
            for m in &inputs[1..] {
                let output = modmul_outputs_iter.next().unwrap();
                test_modmul_bignum(a.clone(), b.clone(), m.clone(), output)?;
            }
        }
    }

    Ok(())
}

#[test]
fn test_modexp_bignum_all() -> Result<()> {
    let exp_bit_sizes = vec![2, 9, 11, 16];

    for bit_size in &BIT_SIZES_TO_TEST[3..7] {
        for exp_bit_size in &exp_bit_sizes {
            let b = gen_bignum(*bit_size);
            let e = gen_bignum(*exp_bit_size);
            let m = gen_bignum(*bit_size);
            if !m.is_zero() {
                let output = b.clone().modpow(&e, &m);
                test_modexp_bignum(b, e, m, output)?;
            }

            let b = max_bignum(*bit_size);
            let e = max_bignum(*exp_bit_size);
            let m = max_bignum(*bit_size);
            if !m.is_zero() {
                let output = b.modpow(&e, &m);
                test_modexp_bignum(b, e, m, output)?;
            }
        }
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let modexp_outputs = test_data_biguint(TEST_DATA_MODEXP_OUTPUTS);
    let mut modexp_outputs_iter = modexp_outputs.into_iter();
    for b in &inputs[..9] {
        // Include only smaller exponents, to keep tests from becoming too slow.
        for e in &inputs[..6] {
            for m in &inputs[..9] {
                let output = modexp_outputs_iter.next().unwrap();
                test_modexp_bignum(b.clone(), e.clone(), m.clone(), output)?;
            }
        }
    }

    Ok(())
}

#[test]
#[ignore] // Too slow to run on CI.
fn test_modexp_bignum_all_full() -> Result<()> {
    // Only test smaller values for exponent.
    let exp_bit_sizes = vec![2, 100, 127, 128, 129];

    for bit_size in &BIT_SIZES_TO_TEST[3..14] {
        for exp_bit_size in &exp_bit_sizes {
            let b = gen_bignum(*bit_size);
            let e = gen_bignum(*exp_bit_size);
            let m = gen_bignum(*bit_size);
            if !m.is_zero() {
                let output = b.clone().modpow(&e, &m);
                test_modexp_bignum(b, e, m, output)?;
            }

            let b = max_bignum(*bit_size);
            let e = max_bignum(*exp_bit_size);
            let m = max_bignum(*bit_size);
            if !m.is_zero() {
                let output = b.modpow(&e, &m);
                test_modexp_bignum(b, e, m, output)?;
            }
        }
    }

    let inputs = test_data_biguint(TEST_DATA_BIGNUM_INPUTS);
    let modexp_outputs = test_data_biguint(TEST_DATA_MODEXP_OUTPUTS_FULL);
    let mut modexp_outputs_iter = modexp_outputs.into_iter();
    for b in &inputs {
        // Include only smaller exponents, to keep tests from becoming too slow.
        for e in &inputs[..7] {
            for m in &inputs {
                let output = modexp_outputs_iter.next().unwrap();
                test_modexp_bignum(b.clone(), e.clone(), m.clone(), output)?;
            }
        }
    }

    Ok(())
}
