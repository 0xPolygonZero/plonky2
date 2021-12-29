use keccak_hash::keccak;

use crate::hash::hash_types::{BytesHash, RichField};
use crate::hash::hashing::{PlonkyPermutation, SPONGE_WIDTH};
use crate::plonk::config::Hasher;
use crate::util::serialization::Buffer;

/// Keccak-256 permutation used in the challenger.
pub struct KeccakPermutation;
impl<F: RichField> PlonkyPermutation<F> for KeccakPermutation {
    fn permute(input: [F; SPONGE_WIDTH]) -> [F; SPONGE_WIDTH] {
        // Fill a byte array with the little-endian representation of the field array.
        let mut buffer = [0u8; SPONGE_WIDTH * std::mem::size_of::<u64>()];
        for i in 0..SPONGE_WIDTH {
            buffer[i * std::mem::size_of::<F>()..(i + 1) * std::mem::size_of::<F>()]
                .copy_from_slice(&input[i].to_canonical_u64().to_le_bytes());
        }
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
            permutated_input[i] = F::from_noncanonical_u64(u64::from_le_bytes(
                permutated_input_bytes
                    [i * std::mem::size_of::<F>()..(i + 1) * std::mem::size_of::<F>()]
                    .try_into()
                    .unwrap(),
            ));
        }
        permutated_input
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
