//! Generates random constants using ChaCha20, seeded with zero.

use plonky2::field::crandall_field::CrandallField;
use plonky2::field::field_types::{Field, PrimeField};
use plonky2::field::goldilocks_field::GoldilocksField;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

// We will sample from CrandallField, which is slightly larger than GoldilocksField, then verify
// that each constant also fits in GoldilocksField.
type F = CrandallField;

// const N: usize = 101; // For GMiMC
// const N: usize = 8 * 30; // For Posiedon-8
const N: usize = 12 * 30; // For Posiedon-12

pub(crate) fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    let mut constants = [F::ZERO; N];
    for i in 0..N {
        constants[i] = F::rand_from_rng(&mut rng);
        // Make sure the constant fits in the smaller field (Goldilocks) as well. If so, we also
        // have random numbers in the smaller field. This may be viewed as rejection sampling,
        // except that we never encounter a rejection in practice, so we don't bother handling it.
        assert!(constants[i].to_canonical_u64() < GoldilocksField::ORDER);
    }

    // Print the constants in the format we prefer in our code.
    for chunk in constants.chunks(4) {
        for (i, c) in chunk.iter().enumerate() {
            print!("{:#018x},", c.to_canonical_u64());
            if i != chunk.len() - 1 {
                print!(" ");
            }
        }
        println!();
    }
}
