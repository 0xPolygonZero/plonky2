use anyhow::Result;
use ethereum_types::U256;
use itertools::Itertools;
use num::BigUint;
use num_bigint::RandBigInt;
use rand::Rng;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::util::{biguint_to_mem_vec, mem_vec_to_biguint};

fn pack_bignums(biguints: &[BigUint], length: usize) -> Vec<U256> {
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
    (a.bits() as usize) / 128 + 1
}

fn gen_two_bignums_ordered(bit_size: usize) -> (BigUint, BigUint) {
    let mut rng = rand::thread_rng();
    let (a, b) = {
        let a = rng.gen_biguint(bit_size as u64);
        let b = rng.gen_biguint(bit_size as u64);
        (a.clone().max(b.clone()), a.min(b))
    };

    (a, b)
}

fn prepare_bignum(bit_size: usize) -> (BigUint, U256, Vec<U256>) {
    let a = gen_bignum(bit_size);
    let length: U256 = bignum_len(&a).into();
    let a_limbs = biguint_to_mem_vec(a.clone());

    (a, length, a_limbs)
}

fn prepare_two_bignums(bit_size: usize) -> (BigUint, BigUint, U256, Vec<U256>) {
    let (a, b) = gen_two_bignums_ordered(bit_size);
    let length: U256 = bignum_len(&a).into();
    let memory = pack_bignums(&[a.clone(), b.clone()], length.try_into().unwrap());

    (a, b, length, memory)
}

#[test]
fn test_shr_bignum() -> Result<()> {
    let (a, length, memory) = prepare_bignum(1000);

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

#[test]
fn test_iszero_bignum() -> Result<()> {
    let (_a, length, memory) = prepare_bignum(1000);

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

#[test]
fn test_ge_bignum() -> Result<()> {
    let (_a, _b, length, memory) = prepare_two_bignums(1000);

    let retdest = 0xDEADBEEFu32.into();
    let ge_bignum = KERNEL.global_labels["ge_bignum"];

    let a_start_loc = 0.into();
    let b_start_loc = length;

    // Test with a > b.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(ge_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory.clone());
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, U256::one());

    // Swap a and b, to test the less-than case.
    let mut initial_stack: Vec<U256> = vec![length, b_start_loc, a_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(ge_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, 0.into());

    Ok(())
}

#[test]
fn test_add_bignum() -> Result<()> {
    let (a, b, length, memory) = prepare_two_bignums(1000);

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

#[test]
fn test_addmul_bignum() -> Result<()> {
    let mut rng = rand::thread_rng();
    let (a, b, length, mut memory) = prepare_two_bignums(1000);
    let len: usize = length.try_into().unwrap();
    memory.splice(len..len, vec![0.into(); 2].iter().cloned());

    let val: u64 = rng.gen();
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
    let carry = interpreter.stack()[0];
    let mut new_memory = interpreter.get_kernel_general_memory();
    new_memory[len] = carry;
    let actual_result: Vec<_> = new_memory[..expected_result.len()].into();

    // Compare.
    assert_eq!(actual_result, expected_result);

    Ok(())
}

#[test]
fn test_mul_bignum() -> Result<()> {
    let (a, b, length, memory) = prepare_two_bignums(1000);

    // Determine expected product.
    let product = a * b;
    let expected_product: Vec<U256> = biguint_to_mem_vec(product);

    // Output and scratch space locations (initialized as zeroes) follow a and b in memory.
    let a_start_loc = 0.into();
    let b_start_loc = length;
    let output_loc = length * 2;
    let scratch_space = length * 4;

    // Prepare stack.
    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![
        length,
        a_start_loc,
        b_start_loc,
        output_loc,
        scratch_space,
        retdest,
    ];
    initial_stack.reverse();

    // Prepare interpreter.
    let mul_bignum = KERNEL.global_labels["mul_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(mul_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    // Run mul function.
    interpreter.run()?;

    dbg!(interpreter.stack());

    // Determine actual product.
    let new_memory = interpreter.get_kernel_general_memory();
    let output_location: usize = output_loc.try_into().unwrap();
    let actual_product: Vec<_> =
        new_memory[output_location..output_location + expected_product.len()].into();

    assert_eq!(actual_product, expected_product);

    Ok(())
}
