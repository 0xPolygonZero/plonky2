use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

fn u256_to_be_limbs(x: U256) -> Vec<u8> {
    let mut limbs = vec![0; 32];
    x.to_big_endian(&mut limbs);
    limbs
}

fn gen_u256_limbs<R: Rng>(rng: &mut R, num_bits: usize) -> [u64; 4] {
    let remaining = num_bits % 64;
    let top_limb: u64 = rng.gen_range(0..(1 << remaining));
    if num_bits < 64 {
        [top_limb, 0, 0, 0]
    } else if num_bits < 128 {
        [rng.gen(), top_limb, 0, 0]
    } else if num_bits < 192 {
        [rng.gen(), rng.gen(), top_limb, 0]
    } else {
        [rng.gen(), rng.gen(), rng.gen(), top_limb]
    }
}

fn gen_range_u256(max: U256) -> U256 {
    let mut rng = rand::thread_rng();

    let num_bits = max.bits();

    let mut x: U256 = {
        let limbs = gen_u256_limbs(&mut rng, num_bits);
        U256(limbs)
    };
    while x > max {
        x = {
            let limbs = gen_u256_limbs(&mut rng, num_bits);
            U256(limbs)
        };
    }
    x
}

#[test]
fn test_add_bignum() -> Result<()> {
    let max = U256([0, 0, 0, 1u64 << 6]);
    let a: U256 = gen_range_u256(max);
    let b: U256 = gen_range_u256(max);
    let sum = a + b;

    let a_limbs = u256_to_be_limbs(a);
    let b_limbs = u256_to_be_limbs(b);
    let expected_sum = u256_to_be_limbs(sum);

    let a_len = a_limbs.len().into();
    let b_len = b_limbs.len().into();
    let a_start_loc = 0.into();
    let b_start_loc = a_limbs.len().into();
    let memory: Vec<_> = [&a_limbs[..], &b_limbs[..]].concat();

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

#[test]
fn test_sub_bignum() -> Result<()> {
    let max = U256([0, 0, 0, 1u64 << 6]);
    let a: U256 = gen_range_u256(max);
    let b: U256 = gen_range_u256(a - 1);
    let diff = a - b;

    let a_limbs = u256_to_be_limbs(a);
    let b_limbs = u256_to_be_limbs(b);
    let expected_diff = u256_to_be_limbs(diff);

    let a_len = a_limbs.len().into();
    let b_len = b_limbs.len().into();
    let a_start_loc = 0.into();
    let b_start_loc = a_limbs.len().into();
    let memory: Vec<_> = [&a_limbs[..], &b_limbs[..]].concat();

    let retdest = 0xDEADBEEFu32.into();
    let mut initial_stack: Vec<U256> = vec![a_len, b_len, a_start_loc, b_start_loc, retdest];
    initial_stack.reverse();

    let sub_bignum = KERNEL.global_labels["sub_bignum"];
    let mut interpreter = Interpreter::new_with_kernel(sub_bignum, initial_stack);
    interpreter.set_kernel_general_memory(memory);

    interpreter.run()?;

    let new_memory = interpreter.get_kernel_general_memory();
    let actual_diff: Vec<u8> = new_memory[..expected_diff.len()].into();
    assert_eq!(actual_diff, expected_diff);

    Ok(())
}
