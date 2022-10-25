//! Generates random constants using ChaCha20, seeded with zero.

#![allow(clippy::needless_range_loop)]

use plonky2_field::goldilocks_field::GoldilocksField;
use plonky2_field::types::Field64;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const SAMPLE_RANGE_END: u64 = GoldilocksField::ORDER;

const N: usize = 12 * 30; // For Poseidon-12

pub(crate) fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let mut constants = [0u64; N];
    for i in 0..N {
        constants[i] = rng.gen_range(0..SAMPLE_RANGE_END);
    }

    // Print the constants in the format we prefer in our code.
    for chunk in constants.chunks(4) {
        for (i, c) in chunk.iter().enumerate() {
            print!("{c:#018x},");
            if i != chunk.len() - 1 {
                print!(" ");
            }
        }
        println!();
    }
}
