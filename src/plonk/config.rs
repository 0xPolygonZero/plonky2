use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::quadratic::QuadraticExtension;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::RichField;
use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::{compress, hash_n_to_hash, PoseidonPermutation};
use crate::iop::challenger::Challenger;
use crate::plonk::circuit_builder::CircuitBuilder;

pub trait Hasher<F: RichField> {
    /// Size of `Hash` in bytes.
    const HASH_SIZE: usize;
    type Hash: From<Vec<u8>>
        + Into<Vec<u8>>
        + Into<Vec<F>>
        + Into<u64>
        + Copy
        + Clone
        + Debug
        + Eq
        + PartialEq
        + Send
        + Sync
        + Serialize
        + DeserializeOwned;

    fn hash(input: Vec<F>, pad: bool) -> Self::Hash;
    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash;
    fn observe_hash(hash: Self::Hash, challenger: &mut Challenger<F>);
}

#[derive(Copy, Clone)]
pub struct PoseidonHash;
impl<F: RichField> Hasher<F> for PoseidonHash {
    const HASH_SIZE: usize = 4 * 8;
    type Hash = HashOut<F>;

    fn hash(input: Vec<F>, pad: bool) -> Self::Hash {
        hash_n_to_hash::<F, PoseidonPermutation>(input, pad)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        compress(left, right)
    }

    fn observe_hash(hash: Self::Hash, challenger: &mut Challenger<F>) {
        challenger.observe_hash(&hash)
    }
}

impl<F: RichField> AlgebraicHasher<F> for PoseidonHash {}

pub trait AlgebraicHasher<F: RichField>: Hasher<F, Hash = HashOut<F>> {}

pub trait GenericConfig<const D: usize>:
    Debug + Clone + Sync + Sized + Send + Eq + PartialEq
{
    type F: RichField + Extendable<D, Extension = Self::FE>;
    type FE: FieldExtension<D, BaseField = Self::F>;
    type Hasher: Hasher<Self::F>;
    type InnerHasher: AlgebraicHasher<Self::F>;
}

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PoseidonGoldilocksConfig;
impl AlgebraicConfig<2> for PoseidonGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = PoseidonHash;
    type InnerHasher = PoseidonHash;
}
