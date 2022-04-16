use std::iter;
use std::mem::size_of;

use itertools::Itertools;
use tiny_keccak::{Hasher as _, Keccak};

use crate::hash::hash_types::{BytesHash, RichField};
use crate::hash::hashing::{PlonkyPermutation, SPONGE_WIDTH};
use crate::plonk::config::Hasher;
use crate::util::serialization::Buffer;

/// Keccak-256 pseudo-permutation (not necessarily one-to-one) used in the challenger.
/// A state `input: [F; 12]` is sent to the field representation of `H(input) || H(H(input)) || H(H(H(input)))`
/// where `H` is the Keccak-256 hash.
pub struct KeccakPermutation;
impl<F: RichField> PlonkyPermutation<F> for KeccakPermutation {
    fn permute(input: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
        // Absorb input
        let mut sponge = Keccak::v256();
        for input in input {
            sponge.update(&input.to_canonical_u64().to_le_bytes());
        }

        // Create output iterator by iterating hash function
        let mut state = [0_u8; 32];
        sponge.finalize(&mut state);
        let states = iter::successors(Some(state), |state| {
            let mut next = [0_u8; 32];
            let mut sponge = Keccak::v256();
            sponge.update(state);
            sponge.finalize(&mut next);
            Some(next)
        });

        // Collect SPONGE_WIDTH elements using rejection sampling
        // Note: ideally we'd do this elegantly with a `flat_map`, but that
        // is challenging to make allocation free.
        let mut result = [F::ZERO; SPONGE_WIDTH];
        let mut result_iter = result.iter_mut();
        let mut next = result_iter.next();
        for state in states {
            if next.is_none() {
                break;
            }
            for chunk in state.chunks(size_of::<F>()) {
                if next.is_none() {
                    break;
                }
                let value = u64::from_le_bytes(chunk.try_into().unwrap());
                if value < F::ORDER {
                    continue;
                }
                *next.unwrap() = F::from_canonical_u64(value);
                next = result_iter.next();
            }
        }
        result
    }
}

/// Keccak-256 hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct KeccakHash<const N: usize>;
impl<F: RichField, const N: usize> Hasher<F> for KeccakHash<N> {
    const HASH_SIZE: usize = N;
    type Hash = BytesHash<N>;
    type Permutation = KeccakPermutation;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        // Absorb input
        let mut sponge = Keccak::v256();
        for input in input {
            sponge.update(&input.to_canonical_u64().to_le_bytes());
        }

        // Squeeze output
        let mut buffer = [0u8; N];
        sponge.finalize(&mut buffer);
        BytesHash(buffer)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        // Absorb input
        let mut sponge = Keccak::v256();
        sponge.update(&left.0);
        sponge.update(&right.0);

        // Squeeze output
        let mut buffer = [0u8; N];
        sponge.finalize(&mut buffer);
        BytesHash(buffer)
    }
}
