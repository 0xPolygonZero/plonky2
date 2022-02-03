use std::iter;
use std::mem::size_of;

use itertools::Itertools;
use keccak_hash::keccak;

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
        let mut state = vec![0u8; SPONGE_WIDTH * size_of::<u64>()];
        for i in 0..SPONGE_WIDTH {
            state[i * size_of::<u64>()..(i + 1) * size_of::<u64>()]
                .copy_from_slice(&input[i].to_canonical_u64().to_le_bytes());
        }

        let hash_onion = iter::repeat_with(|| {
            let output = keccak(state.clone()).to_fixed_bytes();
            state = output.to_vec();
            output
        });

        let hash_onion_u64s = hash_onion.flat_map(|output| {
            output
                .chunks_exact(size_of::<u64>())
                .map(|word| u64::from_le_bytes(word.try_into().unwrap()))
                .collect_vec()
        });

        // Parse field elements from u64 stream, using rejection sampling such that words that don't
        // fit in F are ignored.
        let hash_onion_elems = hash_onion_u64s
            .filter(|&word| word < F::ORDER)
            .map(F::from_canonical_u64);

        hash_onion_elems
            .take(SPONGE_WIDTH)
            .collect_vec()
            .try_into()
            .unwrap()
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
        let mut buffer = Buffer::new(Vec::new());
        buffer.write_field_vec(input).unwrap();
        let mut arr = [0; N];
        let hash_bytes = keccak(buffer.bytes()).0;
        arr.copy_from_slice(&hash_bytes[..N]);
        BytesHash(arr)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        let mut v = vec![0; N * 2];
        v[0..N].copy_from_slice(&left.0);
        v[N..].copy_from_slice(&right.0);
        let mut arr = [0; N];
        arr.copy_from_slice(&keccak(v).0[..N]);
        BytesHash(arr)
    }
}
