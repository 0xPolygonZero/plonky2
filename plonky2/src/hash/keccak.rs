use std::mem::size_of;

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
        // Use rejection sampling so that if one of the `u64` values in the output is larger than
        // the field order, we increment the nonce and start again.
        'rejection_sampling: for nonce in 0u64.. {
            // Fill a byte array with the little-endian representation of the field array.
            let mut buffer = [0u8; (SPONGE_WIDTH + 1) * size_of::<u64>()];
            for i in 0..SPONGE_WIDTH {
                buffer[i * size_of::<u64>()..(i + 1) * size_of::<u64>()]
                    .copy_from_slice(&input[i].to_canonical_u64().to_le_bytes());
            }
            // Add the nonce at the end of the buffer.
            buffer[SPONGE_WIDTH * size_of::<u64>()..].copy_from_slice(&nonce.to_le_bytes());
            // Concatenate `H(input), H(H(input)), H(H(H(input)))`.
            let permutated_input_bytes = {
                let mut ans = [0u8; 96];
                ans[0..32].copy_from_slice(&keccak(buffer).0);
                ans[32..64].copy_from_slice(&keccak(keccak(buffer).0).0);
                ans[64..96].copy_from_slice(&keccak(keccak(keccak(buffer).0).0).0);
                ans
            };
            // Write the hashed byte array to a field array.
            let mut permutated_input = [F::ZERO; SPONGE_WIDTH];
            for i in 0..SPONGE_WIDTH {
                let perm_u64 = u64::from_le_bytes(
                    permutated_input_bytes[i * size_of::<u64>()..(i + 1) * size_of::<u64>()]
                        .try_into()
                        .unwrap(),
                );
                if perm_u64 >= F::ORDER {
                    // If a value is larger than the field order, we break and start again with a new nonce.
                    continue 'rejection_sampling;
                } else {
                    permutated_input[i] = F::from_canonical_u64(perm_u64);
                }
            }
            return permutated_input;
        }
        panic!("Improbable.")
    }
}

/// Keccak-256 hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct KeccakHash<const N: usize>;
impl<F: RichField, const N: usize> Hasher<F> for KeccakHash<N> {
    const HASH_SIZE: usize = N;
    type Hash = BytesHash<N>;
    type Permutation = KeccakPermutation;

    fn hash(input: Vec<F>, _pad: bool) -> Self::Hash {
        let mut buffer = Buffer::new(Vec::new());
        buffer.write_field_vec(&input).unwrap();
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
