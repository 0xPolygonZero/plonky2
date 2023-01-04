use anyhow::Result;
use ethereum_types::U256;
use num::Signed;
use num_bigint::{BigUint, RandBigInt};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::biguint_to_le_limbs;

#[test]
fn test_ge_bignum() -> Result<()> {
    let mut rng = rand::thread_rng();
    let a: BigUint = rng.gen_bigint(1000).abs().to_biguint().unwrap();
    let b: BigUint = rng.gen_bigint(1000).abs().to_biguint().unwrap();

    let a_limbs = biguint_to_le_limbs(a);
    let b_limbs = biguint_to_le_limbs(b);
    let length: U256 = a_limbs.len().into();

    let memory: Vec<U256> = [&a_limbs[..], &b_limbs[..]]
        .concat()
        .iter()
        .map(|&x| x.into())
        .collect();
    let a_start_loc = 0.into();
    let b_start_loc = length;

    let retdest = 0xDEADBEEFu32.into();
    let ge_bignum = KERNEL.global_labels["ge_bignum"];

    // Test with a > b.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(ge_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory.clone());
    interpreter.run()?;
    dbg!(interpreter.stack());
    dbg!(interpreter.get_kernel_general_memory());
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
    let mut rng = rand::thread_rng();
    let a: BigUint = rng.gen_bigint(1000).abs().to_biguint().unwrap();
    let b = rng.gen_bigint(1000).abs().to_biguint().unwrap();
    let sum = a.clone() + b.clone();

    let a_limbs = biguint_to_le_limbs(a);
    let b_limbs = biguint_to_le_limbs(b);
    let expected_sum: Vec<U256> = biguint_to_le_limbs(sum).iter().map(|&x| x.into()).collect();
    let length: U256 = a_limbs.len().into();

    let memory: Vec<U256> = [&a_limbs[..], &b_limbs[..]]
        .concat()
        .iter()
        .map(|&x| x.into())
        .collect();
    let a_start_loc = 0.into();
    let b_start_loc = length;

    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();

    let add_bignum = KERNEL.global_labels["add_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(add_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    interpreter.run()?;

    dbg!(interpreter.stack());

    let new_memory = interpreter.get_kernel_general_memory();
    dbg!(new_memory.clone());
    let actual_sum: Vec<_> = new_memory[..expected_sum.len()].into();
    assert_eq!(actual_sum, expected_sum);

    Ok(())
}

#[test]
fn test_mul_bignum() -> Result<()> {
    let mut rng = rand::thread_rng();
    let a: BigUint = rng.gen_bigint(1000).abs().to_biguint().unwrap();
    let b = rng.gen_bigint(1000).abs().to_biguint().unwrap();
    let product = a.clone() * b.clone();

    let a_limbs = biguint_to_le_limbs(a);
    let b_limbs = biguint_to_le_limbs(b);
    let expected_product: Vec<U256> = biguint_to_le_limbs(product)
        .iter()
        .map(|&x| x.into())
        .collect();
    let length: U256 = a_limbs.len().into();

    let memory: Vec<U256> = [&a_limbs[..], &b_limbs[..]]
        .concat()
        .iter()
        .map(|&x| x.into())
        .collect();
    let a_start_loc = 0.into();
    let b_start_loc = length;
    let output_loc = length * U256::from(2);
    let scratch_space = length * U256::from(4);

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

    let mul_bignum = KERNEL.global_labels["mul_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(mul_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    interpreter.run()?;

    dbg!(interpreter.stack());

    let new_memory = interpreter.get_kernel_general_memory();
    dbg!(new_memory.clone());
    let actual_product: Vec<_> = new_memory[..expected_product.len()].into();
    assert_eq!(actual_product, expected_product);

    Ok(())
}
