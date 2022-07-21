use keccak::f1600 as keccak;
use sha2::{Digest, Sha256};

use crate::hash::hash_types::{BytesHash, RichField};
use crate::hash::hashing::{PlonkyPermutation, SPONGE_WIDTH};
use crate::plonk::config::Hasher;

/// Keccak sponge pseudo-permutation (not necessarily one-to-one) used in the challenger.
/// Here, we use the same "hash onion" technique used in `KeccakPermuation`, but instead using the keccak
/// sponge permutation
pub struct KeccakSpongePermutation;
impl<F: RichField> PlonkyPermutation<F> for KeccakSpongePermutation {
    fn permute(input: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
        let mut state = [0u64; 25];
        let mut res = [F::ZERO; SPONGE_WIDTH];

        // absorb input
        for i in 0..SPONGE_WIDTH {
            state[i] = input[i].to_canonical_u64();
        }

        // keep squeezingu until we have SPONGE_WIDTH words that fit in the field
        let mut elems = 0;
        while elems < SPONGE_WIDTH {
            keccak(&mut state);

            for i in 0..SPONGE_WIDTH {
                let word = state[i];
                if word < F::ORDER {
                    res[elems] = F::from_canonical_u64(word);
                    elems += 1;
                }
            }
        }

        res
    }
}

/// Hash config that uses keccak-f1600 as the permutation and sha256 as the hash
/// Note: N must be less than or equal to 32
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct KeccakSpongeSha256Hasher<const N: usize>;
impl<F: RichField, const N: usize> Hasher<F> for KeccakSpongeSha256Hasher<N> {
    const HASH_SIZE: usize = N;
    type Hash = BytesHash<N>;
    type Permutation = KeccakSpongePermutation;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        let mut res = [0; N];
        let mut hasher = Sha256::new();
        for &elem in input.iter() {
            hasher.update(elem.to_canonical_u64().to_le_bytes());
        }
        let hash = hasher.finalize();

        res.copy_from_slice(&hash[..N]);
        BytesHash(res)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        let mut res = [0; N];
        let mut hasher = Sha256::new();

        hasher.update(&left.0);
        hasher.update(&right.0);
        let hash = hasher.finalize();

        res.copy_from_slice(&hash[..N]);
        BytesHash(res)
    }
}
