use anyhow::Result;
use ethereum_types::U256;
use num::{BigUint, Signed};
use num_bigint::RandBigInt;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::util::biguint_to_mem_vec;

fn prepare_two_bignums(bit_size: usize) -> (BigUint, BigUint, U256, Vec<U256>) {
    let mut rng = rand::thread_rng();
    let (a, b) = {
        let a = rng.gen_bigint(bit_size as u64).abs().to_biguint().unwrap();
        let b = rng.gen_bigint(bit_size as u64).abs().to_biguint().unwrap();
        (a.clone().max(b.clone()), a.min(b))
    };

    let a_limbs = biguint_to_mem_vec(a.clone());
    let b_limbs = biguint_to_mem_vec(b.clone());
    let length: U256 = a_limbs.len().into();

    let memory: Vec<U256> = [&a_limbs[..], &b_limbs[..]].concat();

    (a, b, length, memory)
}

fn prepare_three_bignums(
    bit_size: usize,
) -> (BigUint, BigUint, BigUint, U256, Vec<U256>) {
    let mut rng = rand::thread_rng();
    let (a, b) = {
        let a = rng.gen_bigint(bit_size as u64).abs().to_biguint().unwrap();
        let b = rng.gen_bigint(bit_size as u64).abs().to_biguint().unwrap();
        (a.clone().max(b.clone()), a.min(b))
    };

    let m = rng.gen_bigint(bit_size as u64).abs().to_biguint().unwrap();

    let a_limbs = biguint_to_mem_vec(a.clone());
    let mut b_limbs = biguint_to_mem_vec(b.clone());
    let mut m_limbs = biguint_to_mem_vec(m.clone());
    let length: U256 = a_limbs.len().max(m_limbs.len()).into();
    b_limbs.resize(length.as_usize(), U256::zero());
    m_limbs.resize(length.as_usize(), U256::zero());

    let memory: Vec<U256> = [&a_limbs[..], &b_limbs[..], &m_limbs[..]].concat();

    (
        a,
        b,
        m,
        length,
        memory,
    )
}

#[test]
fn test_iszero_bignum() -> Result<()> {
    let (_a, _b, length, mut memory) = prepare_two_bignums(1000);

    let retdest = 0xDEADBEEFu32.into();
    let iszero_bignum = KERNEL.global_labels["iszero_bignum"];

    let a_start_loc = 0.into();
    let b_start_loc = length;

    memory.splice(b_start_loc.as_usize()..(b_start_loc + length).as_usize(), vec![U256::zero(); length.as_usize()].iter().cloned());

    // Test with a > 0.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(iszero_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory.clone());
    interpreter.run()?;
    let result = interpreter.stack()[0];
    assert_eq!(result, U256::zero());

    // Test with a == 0.
    let mut initial_stack: Vec<U256> = vec![length, b_start_loc, retdest];
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
    assert_eq!(result, U256::zero());

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

    // Determine actual product.
    let new_memory = interpreter.get_kernel_general_memory();
    let output_location: usize = output_loc.try_into().unwrap();
    let actual_product: Vec<_> =
        new_memory[output_location..output_location + expected_product.len()].into();

    assert_eq!(actual_product, expected_product);

    Ok(())
}

#[test]
fn test_modmul_bignum() -> Result<()> {
    let (a, b, m, length, mut memory) =
        prepare_three_bignums(1000);

    let len = length.as_usize();
    memory.resize(len * 10, 0.into());

    // Determine expected result.
    let result = (a * b) % m;
    let expected_result: Vec<U256> = biguint_to_mem_vec(result);

    // Output and scratch space locations (initialized as zeroes) follow a and b in memory.
    let a_start_loc = 0.into();
    let b_start_loc = length;
    let m_start_loc = length * 2;
    let output_loc = length * 3;
    let scratch_1 = length * 4;
    let scratch_2 = length * 5;
    let scratch_3 = length * 7;
    let scratch_4 = length * 9;

    // Prepare stack.
    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![
        length,
        a_start_loc,
        b_start_loc,
        m_start_loc,
        output_loc,
        scratch_1,
        scratch_2,
        scratch_3,
        scratch_4,
        retdest,
    ];
    initial_stack.reverse();

    // Prepare interpreter.
    let modmul_bignum = KERNEL.global_labels["modmul_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(modmul_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    // Run modmul function.
    interpreter.run()?;

    // Determine actual result.
    let new_memory = interpreter.get_kernel_general_memory();
    let output_location: usize = output_loc.try_into().unwrap();
    let actual_result: Vec<_> =
        new_memory[output_location..output_location + expected_result.len()].into();

    assert_eq!(actual_result, expected_result);

    Ok(())
}
