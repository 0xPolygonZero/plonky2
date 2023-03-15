use std::f32::MIN;

use anyhow::Result;
use ethereum_types::U256;
use itertools::Itertools;
use num::{BigUint, One, Zero};
use num_bigint::RandBigInt;
use plonky2_util::ceil_div_usize;
use rand::Rng;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::util::{biguint_to_mem_vec, mem_vec_to_biguint, u256_to_biguint};

const BIGNUM_LIMB_BITS: usize = 128;
const MINUS_ONE: U256 = U256::MAX;

fn test_data() -> Vec<Vec<BigUint>> {
    let unary_op_inputs = vec![0u8.into(), 1u8.into(), 2u8.into()];

    let shr_outputs = vec![0u8.into(), 0u8.into(), 1u8.into()];
    let iszero_outputs = vec![1u8.into(), 0u8.into(), 0u8.into()];

    let binary_op_first_inputs = vec![0u8.into(), 1u8.into(), 2u8.into()];
    let binary_op_second_inputs = vec![0u8.into(), 2u8.into(), 1u8.into()];

    let cmp_outputs = vec![0u8.into(), u256_to_biguint(MINUS_ONE), 1u8.into()];
    let add_outputs = vec![0u8.into(), 3u8.into(), 3u8.into()];
    let addmul_outputs = vec![0u8.into(), 2u8.into(), 4u8.into()];
    let mul_outputs = vec![0u8.into(), 2u8.into(), 2u8.into()];

    vec![
        unary_op_inputs,
        shr_outputs,
        iszero_outputs,
        binary_op_first_inputs,
        binary_op_second_inputs,
        cmp_outputs,
        add_outputs,
        addmul_outputs,
        mul_outputs,
    ]
}

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

fn bignum_len(a: &BigUint) -> usize {
    ceil_div_usize(a.bits() as usize, BIGNUM_LIMB_BITS)
}

fn gen_two_bignums_ordered(bit_size: usize) -> (BigUint, BigUint) {
    let mut rng = rand::thread_rng();
    let (a, b) = (
        rng.gen_biguint(bit_size as u64),
        rng.gen_biguint(bit_size as u64),
    );
    if b < a {
        (a, b)
    } else {
        (b, a)
    }
}

fn prepare_bignum_random(bit_size: usize) -> (BigUint, U256, Vec<U256>) {
    let a = gen_bignum(bit_size);
    let length: U256 = bignum_len(&a).into();
    let a_limbs = biguint_to_mem_vec(a.clone());

    (a, length, a_limbs)
}

fn prepare_bignum_max(bit_size: usize) -> (BigUint, U256, Vec<U256>) {
    let a = (BigUint::one() << bit_size) - BigUint::one();
    let length: U256 = bignum_len(&a).into();
    let a_limbs = biguint_to_mem_vec(a.clone());

    (a, length, a_limbs)
}

fn prepare_bignum_min(_bit_size: usize) -> (BigUint, U256, Vec<U256>) {
    let a = BigUint::zero();
    let length: U256 = bignum_len(&a).into();
    let a_limbs = biguint_to_mem_vec(a.clone());

    (a, length, a_limbs)
}

fn prepare_two_bignums_random(bit_size: usize) -> (BigUint, BigUint, U256, Vec<U256>) {
    let (a, b) = gen_two_bignums_ordered(bit_size);
    let length: U256 = bignum_len(&a).into();
    let memory = pad_bignums(&[a.clone(), b.clone()], length.try_into().unwrap());

    (a, b, length, memory)
}

fn prepare_two_bignums_max(bit_size: usize) -> (BigUint, BigUint, U256, Vec<U256>) {
    let a = (BigUint::one() << bit_size) - BigUint::one();
    let b = (BigUint::one() << bit_size) - BigUint::from(2u8);
    let length: U256 = bignum_len(&a).into();
    let memory = pad_bignums(&[a.clone(), b.clone()], length.try_into().unwrap());

    (a, b, length, memory)
}

fn prepare_two_bignums_min(_bit_size: usize) -> (BigUint, BigUint, U256, Vec<U256>) {
    let a = BigUint::one();
    let b = BigUint::zero();
    let length: U256 = bignum_len(&a).into();
    let memory = pad_bignums(&[a.clone(), b.clone()], length.try_into().unwrap());

    (a, b, length, memory)
}

fn prepare_two_bignums_diff(bit_size: usize) -> (BigUint, BigUint, U256, Vec<U256>) {
    let a = BigUint::one() << (bit_size - 1);
    let b = BigUint::zero();
    let length: U256 = bignum_len(&a).into();
    let memory = pad_bignums(&[a.clone(), b.clone()], length.try_into().unwrap());

    (a, b, length, memory)
}

fn prepare_two_bignums_zero(_bit_size: usize) -> (BigUint, BigUint, U256, Vec<U256>) {
    let a = BigUint::zero();
    let b = BigUint::zero();
    let length: U256 = bignum_len(&a).into();
    let memory = pad_bignums(&[a.clone(), b.clone()], length.try_into().unwrap());

    (a, b, length, memory)
}

fn test_shr_bignum<F>(prepare_bignum_fn: &F) -> Result<()>
where
    F: Fn(usize) -> (BigUint, U256, Vec<U256>),
{
    let (a, length, memory) = prepare_bignum_fn(1000);

    let halved = a >> 1;

    let retdest = 0xDEADBEEFu32.into();
    let shr_bignum = KERNEL.global_labels["shr_bignum"];

    let a_start_loc = 0.into();

    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(shr_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);
    interpreter.run()?;

    let new_memory = interpreter.get_kernel_general_memory();
    let new_a = mem_vec_to_biguint(&new_memory[0..length.as_usize()]);
    assert_eq!(new_a, halved);

    Ok(())
}

fn test_iszero_bignum<F>(prepare_bignum_fn: &F) -> Result<()>
where
    F: Fn(usize) -> (BigUint, U256, Vec<U256>),
{
    let (length, memory) = {
        let (mut a, mut length, mut memory) = prepare_bignum_fn(1000);
        while a == BigUint::zero() {
            (a, length, memory) = prepare_bignum_fn(1000);
        }
        (length, memory)
    };

    let retdest = 0xDEADBEEFu32.into();
    let iszero_bignum = KERNEL.global_labels["iszero_bignum"];

    let a_start_loc = 0.into();

    // Test with a > 0.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(iszero_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory.clone());
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, 0.into());

    let memory = vec![0.into(); memory.len()];

    // Test with a == 0.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(iszero_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, U256::one());

    Ok(())
}

fn test_cmp_bignum<F>(prepare_two_bignums_fn: &F) -> Result<()>
where
    F: Fn(usize) -> (BigUint, BigUint, U256, Vec<U256>),
{
    let (_a, _b, length, memory) = prepare_two_bignums_fn(1000);

    let retdest = 0xDEADBEEFu32.into();
    let cmp_bignum = KERNEL.global_labels["cmp_bignum"];

    let a_start_loc = 0.into();
    let b_start_loc = length;

    // Test with a > b.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(cmp_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory.clone());
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, U256::one());

    // Swap a and b, to test the less-than case.
    let mut initial_stack: Vec<U256> = vec![length, b_start_loc, a_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(cmp_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory.clone());
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, MINUS_ONE);

    // Test equal case.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, a_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(cmp_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, U256::zero());

    Ok(())
}

fn test_add_bignum<F>(prepare_two_bignums_fn: &F) -> Result<()>
where
    F: Fn(usize) -> (BigUint, BigUint, U256, Vec<U256>),
{
    let (a, b, length, memory) = prepare_two_bignums_fn(1000);

    // Determine expected sum.
    let sum = a + b;
    let expected_sum: Vec<U256> = biguint_to_mem_vec(sum);

    let a_start_loc = 0.into();
    let b_start_loc = length;

    // Prepare stack.
    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();

    // Prepare interpreter.
    let add_bignum = KERNEL.global_labels["add_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(add_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    // Run add function.
    interpreter.run()?;

    // Determine actual sum.
    let new_memory = interpreter.get_kernel_general_memory();
    let actual_sum: Vec<_> = new_memory[..expected_sum.len()].into();

    // Compare.
    assert_eq!(actual_sum, expected_sum);

    Ok(())
}

fn test_addmul_bignum<F>(prepare_two_bignums_fn: &F) -> Result<()>
where
    F: Fn(usize) -> (BigUint, BigUint, U256, Vec<U256>),
{
    let mut rng = rand::thread_rng();
    let (a, b, length, mut memory) = prepare_two_bignums_fn(1000);
    let len: usize = length.try_into().unwrap();
    memory.splice(len..len, vec![0.into(); 2].iter().cloned());

    let val: u128 = rng.gen();
    let val_u256 = U256::from(val);

    // Determine expected result.
    let result = a + b * BigUint::from(val);
    let expected_result: Vec<U256> = biguint_to_mem_vec(result);

    let a_start_loc = 0.into();
    let b_start_loc = length + 2;

    // Prepare stack.
    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, val_u256, retdest];
    initial_stack.reverse();

    // Prepare interpreter.
    let addmul_bignum = KERNEL.global_labels["addmul_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(addmul_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    // Run add function.
    interpreter.run()?;

    // Determine actual result.
    let carry_limb = interpreter.stack()[0];
    let mut new_memory = interpreter.get_kernel_general_memory();
    new_memory[len] = carry_limb;
    let actual_result: Vec<_> = new_memory[..expected_result.len()].into();

    // Compare.
    assert_eq!(actual_result, expected_result);

    Ok(())
}

fn test_mul_bignum<F>(prepare_two_bignums_fn: &F) -> Result<()>
where
    F: Fn(usize) -> (BigUint, BigUint, U256, Vec<U256>),
{
    let (a, b, length, memory) = prepare_two_bignums_fn(1000);

    // Determine expected product.
    let product = a * b;
    let expected_product: Vec<U256> = biguint_to_mem_vec(product);

    // Output and scratch space locations (initialized as zeroes) follow a and b in memory.
    let a_start_loc = 0.into();
    let b_start_loc = length;
    let output_loc = length * 2;

    // Prepare stack.
    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, output_loc, retdest];
    initial_stack.reverse();

    // Prepare interpreter.
    let mul_bignum = KERNEL.global_labels["mul_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(mul_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    // Run mul function.
    interpreter.run()?;

    // Determine actual product.
    let new_memory = interpreter.get_kernel_general_memory();
    let output_location: usize = output_loc.try_into().unwrap();
    let actual_product: Vec<_> =
        new_memory[output_location..output_location + expected_product.len()].into();

    assert_eq!(actual_product, expected_product);

    Ok(())
}

#[test]
fn test_shr_bignum_all() -> Result<()> {
    test_shr_bignum(&prepare_bignum_random)?;
    test_shr_bignum(&prepare_bignum_max)?;
    test_shr_bignum(&prepare_bignum_min)?;

    Ok(())
}

#[test]
fn test_iszero_bignum_all() -> Result<()> {
    test_iszero_bignum(&prepare_bignum_random)?;
    test_iszero_bignum(&prepare_bignum_max)?;
    // No need to test for min, since it is zero.

    Ok(())
}

#[test]
fn test_cmp_bignum_all() -> Result<()> {
    test_cmp_bignum(&prepare_two_bignums_random)?;
    test_cmp_bignum(&prepare_two_bignums_max)?;
    test_cmp_bignum(&prepare_two_bignums_min)?;
    test_cmp_bignum(&prepare_two_bignums_diff)?;

    Ok(())
}

#[test]
fn test_add_bignum_all() -> Result<()> {
    test_add_bignum(&prepare_two_bignums_random)?;
    test_add_bignum(&prepare_two_bignums_max)?;
    test_add_bignum(&prepare_two_bignums_min)?;
    test_add_bignum(&prepare_two_bignums_diff)?;
    test_add_bignum(&prepare_two_bignums_zero)?;

    Ok(())
}

#[test]
fn test_addmul_bignum_all() -> Result<()> {
    test_addmul_bignum(&prepare_two_bignums_random)?;
    test_addmul_bignum(&prepare_two_bignums_max)?;
    test_addmul_bignum(&prepare_two_bignums_min)?;
    test_addmul_bignum(&prepare_two_bignums_diff)?;
    test_addmul_bignum(&prepare_two_bignums_zero)?;

    Ok(())
}

#[test]
fn test_mul_bignum_all() -> Result<()> {
    test_mul_bignum(&prepare_two_bignums_random)?;
    test_mul_bignum(&prepare_two_bignums_max)?;
    test_mul_bignum(&prepare_two_bignums_min)?;
    test_mul_bignum(&prepare_two_bignums_diff)?;
    test_mul_bignum(&prepare_two_bignums_zero)?;

    Ok(())
}
