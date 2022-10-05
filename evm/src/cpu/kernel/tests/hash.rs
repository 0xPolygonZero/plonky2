use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};
use ripemd::{Digest, Ripemd160};
use sha2::Sha256;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;

/// Standard Sha2 implementation.
fn sha2(input: Vec<u8>) -> U256 {
    let mut hasher = Sha256::new();
    hasher.update(input);
    U256::from(&hasher.finalize()[..])
}

/// Standard RipeMD implementation.
fn ripemd(input: Vec<u8>) -> U256 {
    let mut hasher = Ripemd160::new();
    hasher.update(input);
    U256::from(&hasher.finalize()[..])
}

fn make_random_input() -> Vec<u8> {
    // Generate a random message, between 0 and 9999 bytes.
    let mut rng = thread_rng();
    let num_bytes = rng.gen_range(0..10000);
    (0..num_bytes).map(|_| rng.gen()).collect()
}

fn make_custom_input() -> Vec<u8> {
    // Hardcode a custom message
    vec![
        86, 124, 206, 245, 74, 57, 250, 43, 60, 30, 254, 43, 143, 144, 242, 215, 13, 103, 237, 61,
        90, 105, 123, 250, 189, 181, 110, 192, 227, 57, 145, 46, 221, 238, 7, 181, 146, 111, 209,
        150, 31, 157, 229, 126, 206, 105, 37, 17,
    ]
}

fn make_input_stack(message: Vec<u8>) -> Vec<U256> {
    let mut initial_stack = vec![U256::from(message.len())];
    let bytes: Vec<U256> = message.iter().map(|&x| U256::from(x as u32)).collect();
    initial_stack.extend(bytes);
    initial_stack.push(U256::from_str("0xdeadbeef").unwrap());
    initial_stack.reverse();
    initial_stack
}

fn test_hash(hash_fn_label: &str, standard_implementation: &dyn Fn(Vec<u8>) -> U256) -> Result<()> {
    // Make the input.
    let message_random = make_random_input();
    let message_custom = make_custom_input();

    // Hash the message using a standard implementation.
    let expected_random = standard_implementation(message_random.clone());
    let expected_custom = standard_implementation(message_custom.clone());

    // Load the message onto the stack.
    let initial_stack_random = make_input_stack(message_random);
    let initial_stack_custom = make_input_stack(message_custom);

    // Make the kernel.
    let kernel = combined_kernel();
    let kernel_function = kernel.global_labels[hash_fn_label];

    // Run the kernel code.
    let result_random = run_with_kernel(&kernel, kernel_function, initial_stack_random)?;
    let result_custom = run_with_kernel(&kernel, kernel_function, initial_stack_custom)?;

    // Extract the final output.
    let actual_random = result_random.stack()[0];
    let actual_custom = result_custom.stack()[0];

    // Check that the result is correct.
    assert_eq!(expected_random, actual_random);
    assert_eq!(expected_custom, actual_custom);

    Ok(())
}

#[test]
fn test_sha2() -> Result<()> {
    test_hash("sha2", &sha2)
}

#[test]
fn test_ripemd() -> Result<()> {
    test_hash("ripemd_stack", &ripemd)
}
