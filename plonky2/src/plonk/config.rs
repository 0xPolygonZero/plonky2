use std::fmt::Debug;

use plonky2_field::extension_field::quadratic::QuadraticExtension;
use plonky2_field::extension_field::{Extendable, FieldExtension};
use plonky2_field::goldilocks_field::GoldilocksField;
use serde::{de::DeserializeOwned, Serialize};

use crate::hash::gmimc::GMiMCHash;
use crate::hash::hash_types::HashOut;
use crate::hash::hash_types::RichField;
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
    type Hash: GenericHashOut<F>;

    /// Permutation used in the sponge construction.
    type Permutation: PlonkyPermutation<F>;

    fn hash(input: Vec<F>, pad: bool) -> Self::Hash;
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

/// Configuration trait for "algebraic" configurations, i.e., those using an algebraic hash function
/// in Merkle trees.
/// Same as `GenericConfig` trait but with `InnerHasher: AlgebraicHasher<F>`.
pub trait AlgebraicConfig<const D: usize>:
    Debug + Clone + Sync + Sized + Send + Eq + PartialEq
{
    type F: RichField + Extendable<D, Extension = Self::FE>;
    type FE: FieldExtension<D, BaseField = Self::F>;
    type Hasher: AlgebraicHasher<Self::F>;
    type InnerHasher: AlgebraicHasher<Self::F>;
}

impl<A: AlgebraicConfig<D>, const D: usize> GenericConfig<D> for A {
    type F = <Self as AlgebraicConfig<D>>::F;
    type FE = <Self as AlgebraicConfig<D>>::FE;
    type Hasher = <Self as AlgebraicConfig<D>>::Hasher;
    type InnerHasher = <Self as AlgebraicConfig<D>>::InnerHasher;
}

/// Configuration using Poseidon over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PoseidonGoldilocksConfig;
impl AlgebraicConfig<2> for PoseidonGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = PoseidonHash;
    type InnerHasher = PoseidonHash;
}

/// Configuration using GMiMC over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct GMiMCGoldilocksConfig;
impl AlgebraicConfig<2> for GMiMCGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = GMiMCHash;
    type InnerHasher = GMiMCHash;
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
