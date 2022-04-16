use std::io::Read;
use std::iter;
use blake3::Hasher as Blake3Hasher;
use crate::hash::hash_types::{BytesHash, RichField};
use crate::hash::hashing::{PlonkyPermutation, SPONGE_WIDTH};
use crate::plonk::config::Hasher;

/// Blake3 pseudo-permutation (not necessarily one-to-one) used in the challenger.
pub struct Blake3Permutation;
impl<F: RichField> PlonkyPermutation<F> for Blake3Permutation {
    fn permute(input: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
        // Absorb input
        let mut sponge = Blake3Hasher::new();
        for input in input {
            sponge.update(&input.to_noncanonical_u64().to_le_bytes());
        }

        // Create output iterator using rejection sampling
        let mut squeeze = sponge.finalize_xof();
        let values = iter::repeat_with(|| {
            let mut buffer = [0u8; 8];
            squeeze.read_exact(&mut buffer).unwrap();
            u64::from_le_bytes(buffer)
        })
        .filter(|&word| word < F::ORDER)
        .map(F::from_canonical_u64);

        // Collect SPONGE_WIDTH elements from the output iterator
        let mut result = [F::ZERO; SPONGE_WIDTH];
        for (result, value) in result.iter_mut().zip(values) {
            *result = value;
        }
        result
    }
}

/// Blake3 hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Blake3Hash<const N: usize>;
impl<F: RichField, const N: usize> Hasher<F> for Blake3Hash<N> {
    const HASH_SIZE: usize = N;
    type Hash = BytesHash<N>;
    type Permutation = Blake3Permutation;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        // Absorb input
        let mut sponge = Blake3Hasher::new();
        for input in input {
            sponge.update(&input.to_noncanonical_u64().to_le_bytes());
        }

        // Squeeze output
        let mut squeeze = sponge.finalize_xof();
        let mut buffer = [0u8; N];
        squeeze.read_exact(&mut buffer).unwrap();
        BytesHash(buffer)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        // Absorb input
        let mut sponge = Blake3Hasher::new();
        sponge.update(&left.0);
        sponge.update(&right.0);

        // Squeeze output
        let mut squeeze = sponge.finalize_xof();
        let mut buffer = [0u8; N];
        squeeze.read_exact(&mut buffer).unwrap();
        BytesHash(buffer)
    }
}
