use anyhow::Result;
// use blake2::Blake2b512;
use ethereum_types::U256;
use rand::{thread_rng, Rng};
use ripemd::{Digest, Ripemd160};
use sha2::Sha256;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, InterpreterMemoryInitialization,
};
use crate::memory::segments::Segment::KernelGeneral;

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

fn make_interpreter_setup(
    message: Vec<u8>,
    hash_fn_label: &str,
    hash_input_virt: (usize, usize),
) -> InterpreterMemoryInitialization {
    InterpreterMemoryInitialization {
        label: hash_fn_label.to_string(),
        stack: vec![
            U256::from(hash_input_virt.0),
            U256::from(message.len()),
            U256::from(0xdeadbeefu32),
        ],
        segment: KernelGeneral,
        memory: vec![(
            hash_input_virt.1,
            message.iter().map(|&x| U256::from(x as u32)).collect(),
        )],
    }
}

fn prepare_test<T>(
    hash_fn_label: &str,
    hash_input_virt: (usize, usize),
    standard_implementation: &dyn Fn(Vec<u8>) -> T,
) -> Result<(T, Vec<U256>)> {
    // Make the input.
    let message = make_random_input();

    // Hash the message using a standard implementation.
    let expected = standard_implementation(message.clone());

    // Load the message into the kernel.
    let interpreter_setup = make_interpreter_setup(message, hash_fn_label, hash_input_virt);

    // Run the interpreter
    let result = run_interpreter_with_memory(interpreter_setup).unwrap();

    Ok((expected, result.stack().to_vec()))
}

fn test_hash_256(
    hash_fn_label: &str,
    hash_input_virt: (usize, usize),
    standard_implementation: &dyn Fn(Vec<u8>) -> U256,
) -> Result<()> {
    let (expected, result_stack) =
        prepare_test(hash_fn_label, hash_input_virt, standard_implementation).unwrap();

    // Extract the final output.
    let actual = result_stack[0];

    // Check that the result is correct.
    assert_eq!(expected, actual);

    Ok(())
}

#[test]
fn test_ripemd() -> Result<()> {
    test_hash_256("ripemd", (200, 200), &ripemd)
}

#[test]
fn test_sha2() -> Result<()> {
    test_hash_256("sha2", (0, 1), &sha2)
}

// Since the Blake precompile requires only the blake2_f compression function instead of the full blake2b hash,
// the full hash function is not included in the kernel. To include it, blake2/compression.asm and blake2/main.asm
// must be added to the kernel.

// /// Standard Blake2b implementation.
// fn blake2b(input: Vec<u8>) -> U512 {
//     let mut hasher = Blake2b512::new();
//     hasher.update(input);
//     U512::from(&hasher.finalize()[..])
// }

// fn combine_u256s(hi: U256, lo: U256) -> U512 {
//     U512::from(lo) + (U512::from(hi) << 256)
// }

// fn test_hash_512(
//     hash_fn_label: &str,
//     hash_input_virt: (usize, usize),
//     standard_implementation: &dyn Fn(Vec<u8>) -> U512,
// ) -> Result<()> {
//     let (expected, result_stack) =
//         prepare_test(hash_fn_label, hash_input_virt, standard_implementation).unwrap();

//     // Extract the final output.
//     let actual = combine_u256s(result_stack[0], result_stack[1]);

//     // Check that the result is correct.
//     assert_eq!(expected, actual);

//     Ok(())
// }

// #[test]
// fn test_blake2b() -> Result<()> {
//     test_hash_512("blake2b", (0, 2), &blake2b)
// }
