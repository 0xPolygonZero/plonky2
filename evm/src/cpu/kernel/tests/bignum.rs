use anyhow::Result;
use ethereum_types::U256;
use num::{BigUint, Signed};
use num_bigint::RandBigInt;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::biguint_to_le_limbs;

fn prepare_bignums() -> (BigUint, BigUint, U256, U256, U256, Vec<U256>) {
    let mut rng = rand::thread_rng();
    let (a, b) = {
        let a = rng.gen_bigint(1000).abs().to_biguint().unwrap();
        let b = rng.gen_bigint(1000).abs().to_biguint().unwrap();
        (a.clone().max(b.clone()), a.min(b))
    };

    let a_limbs = biguint_to_le_limbs(a.clone());
    let b_limbs = biguint_to_le_limbs(b.clone());
    let length: U256 = a_limbs.len().into();

    let memory: Vec<U256> = [&a_limbs[..], &b_limbs[..]]
        .concat()
        .iter()
        .map(|&x| x.into())
        .collect();
    let a_start_loc = 0.into();
    let b_start_loc = length;

    (a, b, length, a_start_loc, b_start_loc, memory)
}

#[test]
fn test_ge_bignum() -> Result<()> {
    let (_a, _b, length, a_start_loc, b_start_loc, memory) = prepare_bignums();

    let retdest = 0xDEADBEEFu32.into();
    let ge_bignum = KERNEL.global_labels["ge_bignum"];

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
    let (a, b, length, a_start_loc, b_start_loc, memory) = prepare_bignums();

    // Determine expected sum.
    let sum = a + b;
    let expected_sum: Vec<U256> = biguint_to_le_limbs(sum).iter().map(|&x| x.into()).collect();

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
    let (a, b, length, a_start_loc, b_start_loc, memory) = prepare_bignums();

    // Determine expected product.
    let product = a * b;
    let expected_product: Vec<U256> = biguint_to_le_limbs(product)
        .iter()
        .map(|&x| x.into())
        .collect();

    // Output and scratch space locations (initialized as zeroes) follow a and b in memory.
    let output_loc = length * U256::from(2);
    let scratch_space = length * U256::from(4);

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
    let (a, b, length, a_start_loc, b_start_loc, memory) = prepare_bignums();

    // Determine expected result.
    let result = a * b;
    let expected_result: Vec<U256> = biguint_to_le_limbs(result)
        .iter()
        .map(|&x| x.into())
        .collect();

    // Output and scratch space locations (initialized as zeroes) follow a and b in memory.
    let output_loc = length * U256::from(2);
    let scratch_1 = length * U256::from(3);
    let scratch_2 = length * U256::from(4);
    let scratch_3 = length * U256::from(6);
    let scratch_4 = length * U256::from(8);

    // Prepare stack.
    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![
        length,
        a_start_loc,
        b_start_loc,
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
