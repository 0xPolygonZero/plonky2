use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

fn to_be_limbs(x: U256) -> Vec<u8> {
    let mut cur = x;

    let mut limbs: Vec<u8> = Vec::new();
    while cur > U256::zero() {
        limbs.push((cur % 256).try_into().unwrap());
        cur = cur / 256;
    }

    limbs
}

#[test]
fn test_add_bignum() -> Result<()> {
    let mut rng = rand::thread_rng();
    let a: U256 = U256(rng.gen::<[u64; 4]>());
    let b: U256 = U256(rng.gen::<[u64; 4]>());
    let sum = a + b;

    let a_limbs = to_be_limbs(a);
    let b_limbs = to_be_limbs(b);

    let expected_sum = to_be_limbs(sum);

    let a_len = a_limbs.len().into();
    let b_len = b_limbs.len().into();
    let a_start_loc = 0.into();
    let b_start_loc = a_limbs.len().into();
    let memory: Vec<_> = [&a_limbs[..], &b_limbs[..]].concat().into();

    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![a_len, b_len, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();

    let add_bignum = KERNEL.global_labels["add_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(add_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    interpreter.run()?;

    let new_memory = interpreter.get_kernel_general_memory();
    let actual_sum: Vec<u8> = new_memory[..expected_sum.len()].into();
    assert_eq!(actual_sum, expected_sum);

    Ok(())
}

// #[test]
// fn test_ge_unbounded(x: U256, p: U256) -> Result<()> {
//     let mut initial_stack = vec![U256::from(message.len())];

//     let bytes: Vec<U256> = message.iter().map(|&x| U256::from(x as u32)).collect();
//     initial_stack.extend(bytes);
//     initial_stack.push(U256::from_str("0xdeadbeef").unwrap());
//     initial_stack.reverse();

//     // Make the kernel.
//     let kernel = combined_kernel();
//     let kernel_function = kernel.global_labels["ge_unbounded"];

//     // Run the kernel code.
//     let result_random = run_with_kernel(&kernel, kernel_function, initial_stack_random)?;
//     let result_custom = run_with_kernel(&kernel, kernel_function, initial_stack_custom)?;

//     // Extract the final output.
//     let actual_random = result_random.stack()[0];
//     let actual_custom = result_custom.stack()[0];

//     // Check that the result is correct.
//     assert_eq!(expected_random, actual_random);
//     assert_eq!(expected_custom, actual_custom);

//     Ok(())
// }
