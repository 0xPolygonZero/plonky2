use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::field::extension::quadratic::QuadraticExtension;
use crate::field::extension::{Extendable, FieldExtension};
use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::hash_types::{HashOut, RichField};
use crate::hash::hashing::{PlonkyPermutation, SPONGE_WIDTH};
use crate::hash::keccak::KeccakHash;
use crate::hash::poseidon::PoseidonHash;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;

pub trait GenericHashOut<F: RichField>:
    Copy + Clone + Debug + Eq + PartialEq + Send + Sync + Serialize + DeserializeOwned
{
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;

    fn to_vec(&self) -> Vec<F>;
}

/// Trait for hash functions.
pub trait Hasher<F: RichField>: Sized + Clone + Debug + Eq + PartialEq {
    /// Size of `Hash` in bytes.
    const HASH_SIZE: usize;

    /// Hash Output
    type Hash: GenericHashOut<F>;

    /// Permutation used in the sponge construction.
    type Permutation: PlonkyPermutation<F>;

    /// Hash a message without any padding step. Note that this can enable length-extension attacks.
    /// However, it is still collision-resistant in cases where the input has a fixed length.
    fn hash_no_pad(input: &[F]) -> Self::Hash;

    /// Pad the message using the `pad10*1` rule, then hash it.
    fn hash_pad(input: &[F]) -> Self::Hash {
        let mut padded_input = input.to_vec();
        padded_input.push(F::ONE);
        while (padded_input.len() + 1) % SPONGE_WIDTH != 0 {
            padded_input.push(F::ZERO);
        }
        padded_input.push(F::ONE);
        Self::hash_no_pad(&padded_input)
    }

    /// Hash the slice if necessary to reduce its length to ~256 bits. If it already fits, this is a
    /// no-op.
    fn hash_or_noop(inputs: &[F]) -> Self::Hash {
        if inputs.len() <= 4 {
            let mut inputs_bytes = vec![0u8; Self::HASH_SIZE];
            for i in 0..inputs.len() {
                inputs_bytes[i * 8..(i + 1) * 8]
                    .copy_from_slice(&inputs[i].to_canonical_u64().to_le_bytes());
            }
            Self::Hash::from_bytes(&inputs_bytes)
        } else {
            Self::hash_no_pad(inputs)
        }
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash;
}

/// Trait for algebraic hash functions, built from a permutation using the sponge construction.
pub trait AlgebraicHasher<F: RichField>: Hasher<F, Hash = HashOut<F>> {
    // TODO: Adding a `const WIDTH: usize` here yields a compiler error down the line.
    // Maybe try again in a while.

    /// Circuit to conditionally swap two chunks of the inputs (useful in verifying Merkle proofs),
    /// then apply the permutation.
    fn permute_swapped<const D: usize>(
        inputs: [Target; SPONGE_WIDTH],
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> [Target; SPONGE_WIDTH]
    where
        F: RichField + Extendable<D>;
}

/// Generic configuration trait.
pub trait GenericConfig<const D: usize>:
    Debug + Clone + Sync + Sized + Send + Eq + PartialEq
{
    /// Main field.
    type F: RichField + Extendable<D, Extension = Self::FE>;
    /// Field extension of degree D of the main field.
    type FE: FieldExtension<D, BaseField = Self::F>;
    /// Hash function used for building Merkle trees.
    type Hasher: Hasher<Self::F>;
    /// Algebraic hash function used for the challenger and hashing public inputs.
    type InnerHasher: AlgebraicHasher<Self::F>;
}

/// Configuration using Poseidon over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PoseidonGoldilocksConfig;
impl GenericConfig<2> for PoseidonGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = PoseidonHash;
    type InnerHasher = PoseidonHash;
}

/// Configuration using truncated Keccak over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct KeccakGoldilocksConfig;
impl GenericConfig<2> for KeccakGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = KeccakHash<25>;
    type InnerHasher = PoseidonHash;
}
