use std::convert::TryInto;
use std::fmt::Debug;
use std::io::Cursor;
use std::marker::PhantomData;

use keccak_hash::keccak;
use serde::de::{DeserializeOwned, Error};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::field::extension_field::quadratic::QuadraticExtension;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::{RichField, WIDTH};
use crate::field::goldilocks_field::GoldilocksField;
use crate::gates::poseidon::PoseidonGate;
use crate::hash::gmimc::GMiMC;
use crate::hash::hash_types::HashOut;
use crate::hash::hashing::{compress, hash_n_to_hash, PlonkyPermutation, PoseidonPermutation};
use crate::hash::poseidon::Poseidon;
use crate::iop::challenger::Challenger;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;
use crate::util::serialization::Buffer;

// const WIDTH: usize = 12;

pub trait Hasher<F: RichField>: Sized + Clone + Debug + Eq + PartialEq {
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
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PoseidonHash;
impl<F: RichField> Hasher<F> for PoseidonHash {
    const HASH_SIZE: usize = 4 * 8;
    type Hash = HashOut<F>;

    fn hash(input: Vec<F>, pad: bool) -> Self::Hash {
        hash_n_to_hash::<F, PoseidonPermutation>(input, pad)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        compress::<F, <Self as AlgebraicHasher<F>>::Permutation>(left, right)
    }
}

impl<F: RichField> AlgebraicHasher<F> for PoseidonHash {
    type Permutation = PoseidonPermutation;

    fn permute_swapped<const D: usize>(
        inputs: [Target; WIDTH],
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> [Target; WIDTH]
    where
        F: Extendable<D>,
    {
        let gate_type = PoseidonGate::<F, D>::new();
        let gate = builder.add_gate(gate_type, vec![]);

        let swap_wire = PoseidonGate::<F, D>::WIRE_SWAP;
        let swap_wire = Target::wire(gate, swap_wire);
        builder.connect(swap.target, swap_wire);

        // Route input wires.
        for i in 0..WIDTH {
            let in_wire = PoseidonGate::<F, D>::wire_input(i);
            let in_wire = Target::wire(gate, in_wire);
            builder.connect(inputs[i], in_wire);
        }

        // Collect output wires.
        (0..WIDTH)
            .map(|i| Target::wire(gate, PoseidonGate::<F, D>::wire_output(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

pub trait AlgebraicHasher<F: RichField>: Hasher<F, Hash = HashOut<F>> {
    // TODO: Adding a `const WIDTH: usize` here yields a compiler error down the line.
    // Maybe try again in a while.
    type Permutation: PlonkyPermutation<F>;
    fn permute_swapped<const D: usize>(
        inputs: [Target; WIDTH],
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> [Target; WIDTH]
    where
        F: Extendable<D>;
}

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

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct BytesHash<const N: usize>([u8; N]);
impl<const N: usize> Serialize for BytesHash<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        todo!()
    }
}
impl<'de, const N: usize> Deserialize<'de> for BytesHash<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct KeccakHash<const N: usize>;
impl<F: RichField, const N: usize> Hasher<F> for KeccakHash<N> {
    const HASH_SIZE: usize = N;
    type Hash = BytesHash<N>;

    fn hash(input: Vec<F>, _pad: bool) -> Self::Hash {
        let mut buffer = Buffer::new(Vec::new());
        buffer.write_field_vec(&input).unwrap();
        let mut arr = [0; N];
        arr.copy_from_slice(&keccak(buffer.bytes()).0[..N]);
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

impl<const N: usize> From<Vec<u8>> for BytesHash<N> {
    fn from(v: Vec<u8>) -> Self {
        Self(v.try_into().unwrap())
    }
}

impl<const N: usize> Into<Vec<u8>> for BytesHash<N> {
    fn into(self) -> Vec<u8> {
        self.0.to_vec()
    }
}
impl<const N: usize> Into<u64> for BytesHash<N> {
    fn into(self) -> u64 {
        u64::from_le_bytes(self.0[..8].try_into().unwrap())
    }
}

impl<F: RichField, const N: usize> Into<Vec<F>> for BytesHash<N> {
    fn into(self) -> Vec<F> {
        let n = self.0.len();
        let mut v = self.0.to_vec();
        v.resize(ceil_div_usize(n, 8) * 8, 0);
        let mut buffer = Buffer::new(v);
        buffer.read_field_vec(buffer.len() / 8).unwrap()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct KeccakGoldilocksConfig;
impl GenericConfig<2> for KeccakGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = KeccakHash<25>;
    type InnerHasher = PoseidonHash;
}
