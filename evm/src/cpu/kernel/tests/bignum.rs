use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::{gen_random_u256, u256_to_le_limbs};

#[test]
fn test_ge_bignum_bounded() -> Result<()> {
    let max = U256([0, 0, 0, 1u64 << 6]); // 2^198
    let a: U256 = gen_random_u256(max);
    let b: U256 = gen_random_u256(a - 1);

    let a_limbs = u256_to_le_limbs(a);
    let b_limbs = u256_to_le_limbs(b);

    let length = a_limbs.len().max(b_limbs.len()).into();

    let memory: Vec<U256> = [&a_limbs[..], &b_limbs[..]]
        .concat()
        .iter()
        .map(|&x| x.into())
        .collect();
    let a_start_loc = 0.into();
    let b_start_loc = a_limbs.len().into();

    dbg!(memory.clone());

    let retdest = 0xDEADBEEFu32.into();
    let ge_bignum = KERNEL.global_labels["ge_bignum_bounded"];

    // Test with a > b.
    let mut initial_stack: Vec<U256> = vec![length, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(ge_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory.clone());
    interpreter.run()?;
    dbg!(interpreter.stack());
    dbg!(interpreter.get_kernel_general_memory());
    let _result = interpreter.stack()[0];
    // assert_eq!(result, U256::one());

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

// #[test]
// fn test_add_bignum() -> Result<()> {
//     let max = U256([0, 0, 0, 1u64 << 6]);
//     let a: U256 = gen_random_u256(max);
//     let b: U256 = gen_random_u256(max);
//     let sum = a + b;

//     let a_limbs = u256_to_le_limbs(a);
//     let b_limbs = u256_to_le_limbs(b);
//     let expected_sum = u256_to_le_limbs(sum);

//     let a_len = a_limbs.len().into();
//     let b_len = b_limbs.len().into();
//     let a_start_loc = 0.into();
//     let b_start_loc = a_limbs.len().into();
//     let memory: Vec<_> = [&a_limbs[..], &b_limbs[..]].concat();

//     let retdest = 0xDEADBEEFu32.into();
//     let mut initial_stack: Vec<U256> = vec![a_len, b_len, a_start_loc, b_start_loc, retdest];
//     initial_stack.reverse();

//     let add_bignum = KERNEL.global_labels["add_bignum"];
//     let mut interpreter = Interpreter::new_with_kernel(add_bignum, initial_stack);
//     interpreter.set_kernel_general_memory(memory);

//     interpreter.run()?;

//     let new_memory = interpreter.get_kernel_general_memory();
//     let actual_sum: Vec<u8> = new_memory[..expected_sum.len()].into();
//     assert_eq!(actual_sum, expected_sum);

//     Ok(())
// }

// #[test]
// fn test_sub_bignum() -> Result<()> {
//     let max = U256([0, 0, 0, 1u64 << 6]);
//     let a: U256 = gen_random_u256(max);
//     let b: U256 = gen_random_u256(a - 1);
//     let diff = a - b;

//     let a_limbs = u256_to_le_limbs(a);
//     let b_limbs = u256_to_le_limbs(b);
//     let expected_diff = u256_to_le_limbs(diff);

//     let a_len = a_limbs.len().into();
//     let b_len = b_limbs.len().into();
//     let a_start_loc = 0.into();
//     let b_start_loc = a_limbs.len().into();
//     let memory: Vec<_> = [&a_limbs[..], &b_limbs[..]].concat();

//     let retdest = 0xDEADBEEFu32.into();
//     let mut initial_stack: Vec<U256> = vec![a_len, b_len, a_start_loc, b_start_loc, retdest];
//     initial_stack.reverse();

//     let sub_bignum = KERNEL.global_labels["sub_bignum"];
//     let mut interpreter = Interpreter::new_with_kernel(sub_bignum, initial_stack);
//     interpreter.set_kernel_general_memory(memory);

//     interpreter.run()?;

//     let new_memory = interpreter.get_kernel_general_memory();
//     let actual_diff: Vec<u8> = new_memory[..expected_diff.len()].into();
//     assert_eq!(actual_diff, expected_diff);

//     Ok(())
// }
