use anyhow::Result;
use blake2::Blake2b512;
use ethereum_types::{U256, U512};
use rand::{thread_rng, Rng};
use ripemd::{Digest, Ripemd160};
use sha2::Sha256;

use crate::cpu::kernel::interpreter::InterpreterSetup;
use crate::memory::segments::Segment::KernelGeneral;

/// Standard Blake2b implementation.
fn blake2b(input: Vec<u8>) -> U512 {
    let mut hasher = Blake2b512::new();
    hasher.update(input);
    U512::from(&hasher.finalize()[..])
}

/// Standard RipeMD implementation.
fn ripemd(input: Vec<u8>) -> U256 {
    let mut hasher = Ripemd160::new();
    hasher.update(input);
    U256::from(&hasher.finalize()[..])
}

/// Standard Sha2 implementation.
fn sha2(input: Vec<u8>) -> U256 {
    let mut hasher = Sha256::new();
    hasher.update(input);
    U256::from(&hasher.finalize()[..])
}

fn make_random_input() -> Vec<u8> {
    // Generate a random message, between 0 and 9999 bytes.
    let mut rng = thread_rng();
    let num_bytes = rng.gen_range(0..10000);
    (0..num_bytes).map(|_| rng.gen()).collect()
}

fn combine_u256s(hi: U256, lo: U256) -> U512 {
    let mut result = U512::from(hi);
    result <<= 256;
    result += U512::from(lo);
    result
}

fn prepare_test<T>(
    hash_fn_label: &str,
    hash_input_virt: usize,
    standard_implementation: &dyn Fn(Vec<u8>) -> T,
) -> Result<(T, Vec<U256>)> {
    // Make the input.
    let message_random = make_random_input();

    // Hash the message using a standard implementation.
    let expected_random = standard_implementation(message_random.clone());

    // Load the message into the kernel.
    let interpreter_setup_random = InterpreterSetup {
        label: hash_fn_label.to_string(),
        stack: vec![
            U256::from(hash_input_virt),
            U256::from(message_random.len()),
            U256::from(0xdeadbeefu32),
        ],
        segment: KernelGeneral,
        memory: vec![(
            hash_input_virt,
            message_random
                .iter()
                .map(|&x| U256::from(x as u32))
                .collect(),
        )],
    };

    // Run the interpeter
    let result_random = interpreter_setup_random.run().unwrap();

    Ok((expected_random, result_random.stack().to_vec()))
}

fn test_hash_256(
    hash_fn_label: &str,
    hash_input_virt: usize,
    standard_implementation: &dyn Fn(Vec<u8>) -> U256,
) -> Result<()> {
    let (expected_random, random_stack) =
        prepare_test(hash_fn_label, hash_input_virt, standard_implementation).unwrap();

    // Extract the final output.
    let actual_random = random_stack[0];

    // Check that the result is correct.
    assert_eq!(expected_random, actual_random);

    Ok(())
}

fn test_hash_512(
    hash_fn_label: &str,
    hash_input_virt: usize,
    standard_implementation: &dyn Fn(Vec<u8>) -> U512,
) -> Result<()> {
    let (expected_random, random_stack) =
        prepare_test(hash_fn_label, hash_input_virt, standard_implementation).unwrap();

    // Extract the final output.
    let actual_random = combine_u256s(random_stack[0], random_stack[1]);

    // Check that the result is correct.
    assert_eq!(expected_random, actual_random);

    Ok(())
}

// #[test]
// fn test_blake2b() -> Result<()> {
//     test_hash_512("blake2b", &blake2b)
// }

#[test]
fn test_ripemd() -> Result<()> {
    test_hash_256("ripemd", 136, &ripemd)
}

// #[test]
// fn test_sha2() -> Result<()> {
//     test_hash_256("sha2", &sha2)
// }
