use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};
use ripemd::{Digest, Ripemd160};
use sha2::Sha256;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;

fn to_be_limbs(x: U256) -> (usize, Vec<u8>) {
    let mut len = 0;
    let mut limbs: Vec<u8> = Vec::new();
    while x > U256::zero() {
        len += 1;
        limbs.push((x % 256).try_into().unwrap());
        x = x / 256;
    }

    (len, limbs)
}

#[test]
fn test_add_bignum() -> Result<()> {
    let mut rng = rand::thread_rng();
    let a: U256 = rng.gen();
    let b: U256 = rng.gen();

    let (a_len, a_limbs) = to_be_limbs(a);
    let (b_len, b_limbs) = to_be_limbs(b);

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
