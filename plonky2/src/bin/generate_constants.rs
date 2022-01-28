//! Generates random constants using ChaCha20, seeded with zero.

#![allow(clippy::needless_range_loop)]

use plonky2_field::field_types::PrimeField;
use plonky2_field::goldilocks_field::GoldilocksField;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

// For historical reasons, we sample from 0..0xffffffff70000001, which is slightly larger than the
// range of GoldilocksField, then verify that each constant also fits in GoldilocksField.
const SAMPLE_RANGE_END: u64 = 0xffffffff70000001;

// const N: usize = 8 * 30; // For Posiedon-8
const N: usize = 12 * 30; // For Posiedon-12

pub(crate) fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let mut constants = [0u64; N];
    for i in 0..N {
        constants[i] = rng.gen_range(0..SAMPLE_RANGE_END);
        // Make sure the constant fits in Goldilocks. If so, we also have random numbers in
        // GoldilocksField::ORDER. This may be viewed as rejection sampling, except that we never
        // encounter a rejection in practice, so we don't bother handling it.
        assert!(constants[i] < GoldilocksField::ORDER);
    }

    // Print the constants in the format we prefer in our code.
    for chunk in constants.chunks(4) {
        for (i, c) in chunk.iter().enumerate() {
            print!("{:#018x},", c);
            if i != chunk.len() - 1 {
                print!(" ");
            }
        }
        println!();
    }
}
