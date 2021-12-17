use std::convert::TryInto;
use std::fmt::Debug;

use keccak_hash::{keccak, keccak256};
use serde::{de::DeserializeOwned, Serialize};

use crate::field::extension_field::quadratic::QuadraticExtension;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::RichField;
use crate::field::goldilocks_field::GoldilocksField;
use crate::gates::poseidon::PoseidonGate;
use crate::hash::hash_types::{BytesHash, HashOut};
use crate::hash::hashing::{
    compress, hash_n_to_hash, PlonkyPermutation, PoseidonPermutation, SPONGE_WIDTH,
};
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::serialization::Buffer;

/// Trait for hash functions.
pub trait Hasher<F: RichField>: Sized + Clone + Debug + Eq + PartialEq {
    /// Size of `Hash` in bytes.
    const HASH_SIZE: usize;

    type Hash: From<Vec<u8>>
        + Into<Vec<u8>>
        + Into<Vec<F>>
        + Copy
        + Clone
        + Debug
        + Eq
        + PartialEq
        + Send
        + Sync
        + Serialize
        + DeserializeOwned;

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
        F: Extendable<D>;
}

/// Poseidon hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PoseidonHash;
impl<F: RichField> Hasher<F> for PoseidonHash {
    const HASH_SIZE: usize = 4 * 8;
    type Hash = HashOut<F>;
    type Permutation = PoseidonPermutation;

    fn hash(input: Vec<F>, pad: bool) -> Self::Hash {
        hash_n_to_hash::<F, Self::Permutation>(input, pad)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        compress::<F, Self::Permutation>(left, right)
    }
}

impl<F: RichField> AlgebraicHasher<F> for PoseidonHash {
    fn permute_swapped<const D: usize>(
        inputs: [Target; SPONGE_WIDTH],
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> [Target; SPONGE_WIDTH]
    where
        F: Extendable<D>,
    {
        let gate_type = PoseidonGate::<F, D>::new();
        let gate = builder.add_gate(gate_type, vec![]);

        let swap_wire = PoseidonGate::<F, D>::WIRE_SWAP;
        let swap_wire = Target::wire(gate, swap_wire);
        builder.connect(swap.target, swap_wire);

        // Route input wires.
        for i in 0..SPONGE_WIDTH {
            let in_wire = PoseidonGate::<F, D>::wire_input(i);
            let in_wire = Target::wire(gate, in_wire);
            builder.connect(inputs[i], in_wire);
        }

        // Collect output wires.
        (0..SPONGE_WIDTH)
            .map(|i| Target::wire(gate, PoseidonGate::<F, D>::wire_output(i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

// TODO: Remove width from `GMiMCGate` to make this work.
// #[derive(Copy, Clone, Debug, Eq, PartialEq)]
// pub struct GMiMCHash;
// impl<F: RichField> Hasher<F> for GMiMCHash {
//     const HASH_SIZE: usize = 4 * 8;
//     type Hash = HashOut<F>;
//
//     fn hash(input: Vec<F>, pad: bool) -> Self::Hash {
//         hash_n_to_hash::<F, <Self as AlgebraicHasher<F>>::Permutation>(input, pad)
//     }
//
//     fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
//         compress::<F, <Self as AlgebraicHasher<F>>::Permutation>(left, right)
//     }
// }
//
// impl<F: RichField> AlgebraicHasher<F> for GMiMCHash {
//     type Permutation = GMiMCPermutation;
//
//     fn permute_swapped<const D: usize>(
//         inputs: [Target; WIDTH],
//         swap: BoolTarget,
//         builder: &mut CircuitBuilder<F, D>,
//     ) -> [Target; WIDTH]
//     where
//         F: Extendable<D>,
//     {
//         let gate_type = GMiMCGate::<F, D, W>::new();
//         let gate = builder.add_gate(gate_type, vec![]);
//
//         let swap_wire = GMiMCGate::<F, D, W>::WIRE_SWAP;
//         let swap_wire = Target::wire(gate, swap_wire);
//         builder.connect(swap.target, swap_wire);
//
//         // Route input wires.
//         for i in 0..W {
//             let in_wire = GMiMCGate::<F, D, W>::wire_input(i);
//             let in_wire = Target::wire(gate, in_wire);
//             builder.connect(inputs[i], in_wire);
//         }
//
//         // Collect output wires.
//         (0..W)
//             .map(|i| Target::wire(gate, input: GMiMCGate<F, D, W>::wire_output(i)))
//             .collect::<Vec<_>>()
//             .try_into()
//             .unwrap()
//     }
// }

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
        // Hash the bytes array in-place.
        keccak256(&mut buffer);
        // Write the hashed byte array to a field array.
        let mut hash = [F::ZERO; SPONGE_WIDTH];
        for i in 0..SPONGE_WIDTH {
            hash[i] = F::from_noncanonical_u64(u64::from_le_bytes(
                buffer[i * std::mem::size_of::<F>()..(i + 1) * std::mem::size_of::<F>()]
                    .try_into()
                    .unwrap(),
            ));
        }
        hash
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

/// Configuration using truncated Keccak over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct KeccakGoldilocksConfig;
impl GenericConfig<2> for KeccakGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = KeccakHash<25>;
    type InnerHasher = PoseidonHash;
}
