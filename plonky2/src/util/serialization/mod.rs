#[macro_use]
pub mod generator_serialization;

#[macro_use]
pub mod gate_serialization;

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, sync::Arc, vec, vec::Vec};
use core::convert::Infallible;
use core::fmt::{Debug, Display, Formatter};
use core::mem::size_of;
use core::ops::Range;
#[cfg(feature = "std")]
use std::{collections::BTreeMap, sync::Arc};

pub use gate_serialization::default::DefaultGateSerializer;
pub use gate_serialization::GateSerializer;
pub use generator_serialization::default::DefaultGeneratorSerializer;
pub use generator_serialization::WitnessGeneratorSerializer;
use hashbrown::HashMap;

use crate::field::extension::{Extendable, FieldExtension};
use crate::field::polynomial::PolynomialCoeffs;
use crate::field::types::{Field64, PrimeField64};
use crate::fri::oracle::PolynomialBatch;
use crate::fri::proof::{
    CompressedFriProof, CompressedFriQueryRounds, FriInitialTreeProof, FriInitialTreeProofTarget,
    FriProof, FriProofTarget, FriQueryRound, FriQueryRoundTarget, FriQueryStep, FriQueryStepTarget,
};
use crate::fri::reduction_strategies::FriReductionStrategy;
use crate::fri::{FriConfig, FriParams};
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::gates::gate::GateRef;
use crate::gates::lookup::Lookup;
use crate::gates::selectors::SelectorsInfo;
use crate::hash::hash_types::{HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_proofs::{MerkleProof, MerkleProofTarget};
use crate::hash::merkle_tree::{MerkleCap, MerkleTree};
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::WitnessGeneratorRef;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::wire::Wire;
use crate::plonk::circuit_builder::LookupWire;
use crate::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, ProverCircuitData, ProverOnlyCircuitData,
    VerifierCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use crate::plonk::config::{GenericConfig, GenericHashOut, Hasher};
use crate::plonk::plonk_common::salt_size;
use crate::plonk::proof::{
    CompressedProof, CompressedProofWithPublicInputs, OpeningSet, OpeningSetTarget, Proof,
    ProofTarget, ProofWithPublicInputs, ProofWithPublicInputsTarget,
};

/// A no_std compatible variant of `std::io::Error`
#[derive(Debug)]
pub struct IoError;

impl Display for IoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

/// A no_std compatible variant of `std::io::Result`
pub type IoResult<T> = Result<T, IoError>;

/// A `Read` which is able to report how many bytes are remaining.
pub trait Remaining: Read {
    /// Returns the number of bytes remaining in the buffer.
    fn remaining(&self) -> usize;

    /// Returns whether zero bytes are remaining.
    fn is_empty(&self) -> bool {
        self.remaining() == 0
    }
}

/// Similar to `std::io::Read`, but works with no_std.
pub trait Read {
    /// Reads exactly the length of `bytes` from `self` and writes it to `bytes`.
    fn read_exact(&mut self, bytes: &mut [u8]) -> IoResult<()>;

    /// Reads a `bool` value from `self`.
    #[inline]
    fn read_bool(&mut self) -> IoResult<bool> {
        let i = self.read_u8()?;
        match i {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(IoError),
        }
    }

    /// Reads a `BoolTarget` value from `self`.
    #[inline]
    fn read_target_bool(&mut self) -> IoResult<BoolTarget> {
        Ok(BoolTarget::new_unsafe(self.read_target()?))
    }

    /// Reads a vector of `BoolTarget` from `self`.
    #[inline]
    fn read_target_bool_vec(&mut self) -> IoResult<Vec<BoolTarget>> {
        let length = self.read_usize()?;
        (0..length)
            .map(|_| self.read_target_bool())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Reads a `u8` value from `self`.
    #[inline]
    fn read_u8(&mut self) -> IoResult<u8> {
        let mut buf = [0; size_of::<u8>()];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Reads a `u16` value from `self`.
    #[inline]
    fn read_u16(&mut self) -> IoResult<u16> {
        let mut buf = [0; size_of::<u16>()];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Reads a `u32` value from `self`.
    #[inline]
    fn read_u32(&mut self) -> IoResult<u32> {
        let mut buf = [0; size_of::<u32>()];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Reads a `usize` value from `self`.
    #[inline]
    fn read_usize(&mut self) -> IoResult<usize> {
        let mut buf = [0; core::mem::size_of::<u64>()];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf) as usize)
    }

    /// Reads a vector of `usize` value from `self`.
    #[inline]
    fn read_usize_vec(&mut self) -> IoResult<Vec<usize>> {
        let len = self.read_usize()?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            res.push(self.read_usize()?);
        }

        Ok(res)
    }

    /// Reads a element from the field `F` with size less than `2^64` from `self.`
    #[inline]
    fn read_field<F>(&mut self) -> IoResult<F>
    where
        F: Field64,
    {
        let mut buf = [0; size_of::<u64>()];
        self.read_exact(&mut buf)?;
        Ok(F::from_canonical_u64(u64::from_le_bytes(buf)))
    }

    /// Reads a vector of elements from the field `F` from `self`.
    #[inline]
    fn read_field_vec<F>(&mut self, length: usize) -> IoResult<Vec<F>>
    where
        F: Field64,
    {
        (0..length)
            .map(|_| self.read_field())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Reads an element from the field extension of `F` from `self.`
    #[inline]
    fn read_field_ext<F, const D: usize>(&mut self) -> IoResult<F::Extension>
    where
        F: Field64 + Extendable<D>,
    {
        let mut arr = [F::ZERO; D];
        for a in arr.iter_mut() {
            *a = self.read_field()?;
        }
        Ok(<F::Extension as FieldExtension<D>>::from_basefield_array(
            arr,
        ))
    }

    /// Reads a vector of elements from the field extension of `F` from `self`.
    #[inline]
    fn read_field_ext_vec<F, const D: usize>(
        &mut self,
        length: usize,
    ) -> IoResult<Vec<F::Extension>>
    where
        F: RichField + Extendable<D>,
    {
        (0..length).map(|_| self.read_field_ext::<F, D>()).collect()
    }

    /// Reads a Target from `self.`
    #[inline]
    fn read_target(&mut self) -> IoResult<Target> {
        let is_wire = self.read_bool()?;
        if is_wire {
            let row = self.read_usize()?;
            let column = self.read_usize()?;
            Ok(Target::wire(row, column))
        } else {
            let index = self.read_usize()?;
            Ok(Target::VirtualTarget { index })
        }
    }

    /// Reads an ExtensionTarget from `self`.
    #[inline]
    fn read_target_ext<const D: usize>(&mut self) -> IoResult<ExtensionTarget<D>> {
        let mut res = [Target::wire(0, 0); D];
        for r in res.iter_mut() {
            *r = self.read_target()?;
        }

        Ok(ExtensionTarget(res))
    }

    /// Reads an array of Target from `self`.
    #[inline]
    fn read_target_array<const N: usize>(&mut self) -> IoResult<[Target; N]> {
        (0..N)
            .map(|_| self.read_target())
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.try_into().unwrap())
    }

    /// Reads a vector of Target from `self`.
    #[inline]
    fn read_target_vec(&mut self) -> IoResult<Vec<Target>> {
        let length = self.read_usize()?;
        (0..length)
            .map(|_| self.read_target())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Reads a vector of ExtensionTarget from `self`.
    #[inline]
    fn read_target_ext_vec<const D: usize>(&mut self) -> IoResult<Vec<ExtensionTarget<D>>> {
        let length = self.read_usize()?;
        (0..length)
            .map(|_| self.read_target_ext::<D>())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Reads a hash value from `self`.
    #[inline]
    fn read_hash<F, H>(&mut self) -> IoResult<H::Hash>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let mut buf = vec![0; H::HASH_SIZE];
        self.read_exact(&mut buf)?;
        Ok(H::Hash::from_bytes(&buf))
    }

    /// Reads a HashOutTarget value from `self`.
    #[inline]
    fn read_target_hash(&mut self) -> IoResult<HashOutTarget> {
        let mut elements = [Target::wire(0, 0); 4];
        for e in elements.iter_mut() {
            *e = self.read_target()?;
        }

        Ok(HashOutTarget { elements })
    }

    /// Reads a vector of Hash from `self`.
    #[inline]
    fn read_hash_vec<F, H>(&mut self, length: usize) -> IoResult<Vec<H::Hash>>
    where
        F: RichField,
        H: Hasher<F>,
    {
        (0..length)
            .map(|_| self.read_hash::<F, H>())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Reads a value of type [`MerkleCap`] from `self` with the given `cap_height`.
    #[inline]
    fn read_merkle_cap<F, H>(&mut self, cap_height: usize) -> IoResult<MerkleCap<F, H>>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let cap_length = 1 << cap_height;
        Ok(MerkleCap(
            (0..cap_length)
                .map(|_| self.read_hash::<F, H>())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    /// Reads a value of type [`MerkleCapTarget`] from `self`.
    #[inline]
    fn read_target_merkle_cap(&mut self) -> IoResult<MerkleCapTarget> {
        let length = self.read_usize()?;
        Ok(MerkleCapTarget(
            (0..length)
                .map(|_| self.read_target_hash())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    /// Reads a value of type [`MerkleTree`] from `self`.
    #[inline]
    fn read_merkle_tree<F, H>(&mut self) -> IoResult<MerkleTree<F, H>>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let leaves_len = self.read_usize()?;
        let mut leaves = Vec::with_capacity(leaves_len);
        for _ in 0..leaves_len {
            let leaf_len = self.read_usize()?;
            leaves.push(self.read_field_vec(leaf_len)?);
        }

        let digests_len = self.read_usize()?;
        let digests = self.read_hash_vec::<F, H>(digests_len)?;
        let cap_height = self.read_usize()?;
        let cap = self.read_merkle_cap::<F, H>(cap_height)?;
        Ok(MerkleTree {
            leaves,
            digests,
            cap,
        })
    }

    /// Reads a value of type [`OpeningSet`] from `self` with the given `common_data`.
    #[inline]
    fn read_opening_set<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<OpeningSet<F, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let constants = self.read_field_ext_vec::<F, D>(common_data.num_constants)?;
        let plonk_sigmas = self.read_field_ext_vec::<F, D>(config.num_routed_wires)?;
        let wires = self.read_field_ext_vec::<F, D>(config.num_wires)?;
        let plonk_zs = self.read_field_ext_vec::<F, D>(config.num_challenges)?;
        let plonk_zs_next = self.read_field_ext_vec::<F, D>(config.num_challenges)?;
        let lookup_zs = self.read_field_ext_vec::<F, D>(common_data.num_all_lookup_polys())?;
        let lookup_zs_next = self.read_field_ext_vec::<F, D>(common_data.num_all_lookup_polys())?;
        let partial_products = self
            .read_field_ext_vec::<F, D>(common_data.num_partial_products * config.num_challenges)?;
        let quotient_polys = self.read_field_ext_vec::<F, D>(
            common_data.quotient_degree_factor * config.num_challenges,
        )?;
        Ok(OpeningSet {
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_next,
            partial_products,
            quotient_polys,
            lookup_zs,
            lookup_zs_next,
        })
    }

    /// Reads a value of type [`OpeningSetTarget`] from `self`.
    #[inline]
    fn read_target_opening_set<const D: usize>(&mut self) -> IoResult<OpeningSetTarget<D>> {
        let constants = self.read_target_ext_vec::<D>()?;
        let plonk_sigmas = self.read_target_ext_vec::<D>()?;
        let wires = self.read_target_ext_vec::<D>()?;
        let plonk_zs = self.read_target_ext_vec::<D>()?;
        let plonk_zs_next = self.read_target_ext_vec::<D>()?;
        let lookup_zs = self.read_target_ext_vec::<D>()?;
        let next_lookup_zs = self.read_target_ext_vec::<D>()?;
        let partial_products = self.read_target_ext_vec::<D>()?;
        let quotient_polys = self.read_target_ext_vec::<D>()?;

        Ok(OpeningSetTarget {
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_next,
            lookup_zs,
            next_lookup_zs,
            partial_products,
            quotient_polys,
        })
    }

    /// Reads a value of type [`MerkleProof`] from `self`.
    #[inline]
    fn read_merkle_proof<F, H>(&mut self) -> IoResult<MerkleProof<F, H>>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let length = self.read_u8()?;
        Ok(MerkleProof {
            siblings: (0..length)
                .map(|_| self.read_hash::<F, H>())
                .collect::<Result<_, _>>()?,
        })
    }

    /// Reads a value of type [`MerkleProofTarget`] from `self`.
    #[inline]
    fn read_target_merkle_proof(&mut self) -> IoResult<MerkleProofTarget> {
        let length = self.read_u8()?;
        Ok(MerkleProofTarget {
            siblings: (0..length)
                .map(|_| self.read_target_hash())
                .collect::<Result<_, _>>()?,
        })
    }

    /// Reads a value of type [`FriInitialTreeProof`] from `self` with the given `common_data`.
    #[inline]
    fn read_fri_initial_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<FriInitialTreeProof<F, C::Hasher>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let salt = salt_size(common_data.fri_params.hiding);
        let mut evals_proofs = Vec::with_capacity(4);

        let constants_sigmas_v =
            self.read_field_vec(common_data.num_constants + config.num_routed_wires)?;
        let constants_sigmas_p = self.read_merkle_proof()?;
        evals_proofs.push((constants_sigmas_v, constants_sigmas_p));

        let wires_v = self.read_field_vec(config.num_wires + salt)?;
        let wires_p = self.read_merkle_proof()?;
        evals_proofs.push((wires_v, wires_p));

        let zs_partial_v = self.read_field_vec(
            config.num_challenges
                * (1 + common_data.num_partial_products + common_data.num_lookup_polys)
                + salt,
        )?;
        let zs_partial_p = self.read_merkle_proof()?;
        evals_proofs.push((zs_partial_v, zs_partial_p));

        let quotient_v =
            self.read_field_vec(config.num_challenges * common_data.quotient_degree_factor + salt)?;
        let quotient_p = self.read_merkle_proof()?;
        evals_proofs.push((quotient_v, quotient_p));

        Ok(FriInitialTreeProof { evals_proofs })
    }

    /// Reads a value of type [`FriInitialTreeProofTarget`] from `self`.
    #[inline]
    fn read_target_fri_initial_proof(&mut self) -> IoResult<FriInitialTreeProofTarget> {
        let len = self.read_usize()?;
        let mut evals_proofs = Vec::with_capacity(len);

        for _ in 0..len {
            evals_proofs.push((self.read_target_vec()?, self.read_target_merkle_proof()?));
        }

        Ok(FriInitialTreeProofTarget { evals_proofs })
    }

    /// Reads a value of type [`FriQueryStep`] from `self` with the given `arity` and `compressed`
    /// flag.
    #[inline]
    fn read_fri_query_step<F, C, const D: usize>(
        &mut self,
        arity: usize,
        compressed: bool,
    ) -> IoResult<FriQueryStep<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let evals = self.read_field_ext_vec::<F, D>(arity - usize::from(compressed))?;
        let merkle_proof = self.read_merkle_proof()?;
        Ok(FriQueryStep {
            evals,
            merkle_proof,
        })
    }

    /// Reads a value of type [`FriQueryStepTarget`] from `self`.
    #[inline]
    fn read_target_fri_query_step<const D: usize>(&mut self) -> IoResult<FriQueryStepTarget<D>> {
        let evals = self.read_target_ext_vec::<D>()?;
        let merkle_proof = self.read_target_merkle_proof()?;
        Ok(FriQueryStepTarget {
            evals,
            merkle_proof,
        })
    }

    /// Reads a vector of [`FriQueryRound`]s from `self` with `common_data`.
    #[inline]
    #[allow(clippy::type_complexity)]
    fn read_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<Vec<FriQueryRound<F, C::Hasher, D>>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let mut fqrs = Vec::with_capacity(config.fri_config.num_query_rounds);
        for _ in 0..config.fri_config.num_query_rounds {
            let initial_trees_proof = self.read_fri_initial_proof::<F, C, D>(common_data)?;
            let steps = common_data
                .fri_params
                .reduction_arity_bits
                .iter()
                .map(|&ar| self.read_fri_query_step::<F, C, D>(1 << ar, false))
                .collect::<Result<_, _>>()?;
            fqrs.push(FriQueryRound {
                initial_trees_proof,
                steps,
            })
        }
        Ok(fqrs)
    }

    /// Reads a vector of [`FriQueryRoundTarget`]s from `self`.
    #[inline]
    fn read_target_fri_query_rounds<const D: usize>(
        &mut self,
    ) -> IoResult<Vec<FriQueryRoundTarget<D>>> {
        let num_query_rounds = self.read_usize()?;
        let mut fqrs = Vec::with_capacity(num_query_rounds);
        for _ in 0..num_query_rounds {
            let initial_trees_proof = self.read_target_fri_initial_proof()?;
            let num_steps = self.read_usize()?;
            let steps = (0..num_steps)
                .map(|_| self.read_target_fri_query_step::<D>())
                .collect::<Result<Vec<_>, _>>()?;
            fqrs.push(FriQueryRoundTarget {
                initial_trees_proof,
                steps,
            })
        }
        Ok(fqrs)
    }

    /// Reads a value of type [`FriProof`] from `self` with `common_data`.
    #[inline]
    fn read_fri_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<FriProof<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let commit_phase_merkle_caps = (0..common_data.fri_params.reduction_arity_bits.len())
            .map(|_| self.read_merkle_cap(config.fri_config.cap_height))
            .collect::<Result<Vec<_>, _>>()?;
        let query_round_proofs = self.read_fri_query_rounds::<F, C, D>(common_data)?;
        let final_poly = PolynomialCoeffs::new(
            self.read_field_ext_vec::<F, D>(common_data.fri_params.final_poly_len())?,
        );
        let pow_witness = self.read_field()?;
        Ok(FriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
        })
    }

    /// Reads a value of type [`FriProofTarget`] from `self`.
    #[inline]
    fn read_target_fri_proof<const D: usize>(&mut self) -> IoResult<FriProofTarget<D>> {
        let length = self.read_usize()?;
        let commit_phase_merkle_caps = (0..length)
            .map(|_| self.read_target_merkle_cap())
            .collect::<Result<Vec<_>, _>>()?;
        let query_round_proofs = self.read_target_fri_query_rounds::<D>()?;
        let final_poly = PolynomialCoeffsExtTarget(self.read_target_ext_vec::<D>()?);
        let pow_witness = self.read_target()?;

        Ok(FriProofTarget {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
        })
    }

    fn read_fri_reduction_strategy(&mut self) -> IoResult<FriReductionStrategy> {
        let variant = self.read_u8()?;
        match variant {
            0 => {
                let arities = self.read_usize_vec()?;
                Ok(FriReductionStrategy::Fixed(arities))
            }
            1 => {
                let arity_bits = self.read_usize()?;
                let final_poly_bits = self.read_usize()?;

                Ok(FriReductionStrategy::ConstantArityBits(
                    arity_bits,
                    final_poly_bits,
                ))
            }
            2 => {
                let is_some = self.read_u8()?;
                match is_some {
                    0 => Ok(FriReductionStrategy::MinSize(None)),
                    1 => {
                        let max = self.read_usize()?;
                        Ok(FriReductionStrategy::MinSize(Some(max)))
                    }
                    _ => Err(IoError),
                }
            }
            _ => Err(IoError),
        }
    }

    fn read_fri_config(&mut self) -> IoResult<FriConfig> {
        let rate_bits = self.read_usize()?;
        let cap_height = self.read_usize()?;
        let num_query_rounds = self.read_usize()?;
        let proof_of_work_bits = self.read_u32()?;
        let reduction_strategy = self.read_fri_reduction_strategy()?;

        Ok(FriConfig {
            rate_bits,
            cap_height,
            num_query_rounds,
            proof_of_work_bits,
            reduction_strategy,
        })
    }

    fn read_circuit_config(&mut self) -> IoResult<CircuitConfig> {
        let num_wires = self.read_usize()?;
        let num_routed_wires = self.read_usize()?;
        let num_constants = self.read_usize()?;
        let security_bits = self.read_usize()?;
        let num_challenges = self.read_usize()?;
        let max_quotient_degree_factor = self.read_usize()?;
        let use_base_arithmetic_gate = self.read_bool()?;
        let zero_knowledge = self.read_bool()?;
        let fri_config = self.read_fri_config()?;

        Ok(CircuitConfig {
            num_wires,
            num_routed_wires,
            num_constants,
            security_bits,
            num_challenges,
            max_quotient_degree_factor,
            use_base_arithmetic_gate,
            zero_knowledge,
            fri_config,
        })
    }

    fn read_fri_params(&mut self) -> IoResult<FriParams> {
        let config = self.read_fri_config()?;
        let reduction_arity_bits = self.read_usize_vec()?;
        let degree_bits = self.read_usize()?;
        let hiding = self.read_bool()?;

        Ok(FriParams {
            config,
            reduction_arity_bits,
            degree_bits,
            hiding,
        })
    }

    fn read_gate<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<GateRef<F, D>>;

    fn read_generator<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<WitnessGeneratorRef<F, D>>;

    fn read_selectors_info(&mut self) -> IoResult<SelectorsInfo> {
        let selector_indices = self.read_usize_vec()?;
        let groups_len = self.read_usize()?;
        let mut groups = Vec::with_capacity(groups_len);
        for _ in 0..groups_len {
            let start = self.read_usize()?;
            let end = self.read_usize()?;
            groups.push(Range { start, end });
        }

        Ok(SelectorsInfo {
            selector_indices,
            groups,
        })
    }

    fn read_polynomial_batch<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
    ) -> IoResult<PolynomialBatch<F, C, D>> {
        let poly_len = self.read_usize()?;
        let mut polynomials = Vec::with_capacity(poly_len);
        for _ in 0..poly_len {
            let plen = self.read_usize()?;
            polynomials.push(PolynomialCoeffs::new(self.read_field_vec(plen)?));
        }

        let merkle_tree = self.read_merkle_tree()?;
        let degree_log = self.read_usize()?;
        let rate_bits = self.read_usize()?;
        let blinding = self.read_bool()?;

        Ok(PolynomialBatch {
            polynomials,
            merkle_tree,
            degree_log,
            rate_bits,
            blinding,
        })
    }

    fn read_common_circuit_data<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> IoResult<CommonCircuitData<F, D>> {
        let config = self.read_circuit_config()?;
        let fri_params = self.read_fri_params()?;

        let selectors_info = self.read_selectors_info()?;
        let quotient_degree_factor = self.read_usize()?;
        let num_gate_constraints = self.read_usize()?;
        let num_constants = self.read_usize()?;
        let num_public_inputs = self.read_usize()?;

        let k_is_len = self.read_usize()?;
        let k_is = self.read_field_vec(k_is_len)?;

        let num_partial_products = self.read_usize()?;

        let num_lookup_polys = self.read_usize()?;
        let num_lookup_selectors = self.read_usize()?;
        let length = self.read_usize()?;
        let mut luts = Vec::with_capacity(length);

        for _ in 0..length {
            luts.push(Arc::new(self.read_lut()?));
        }

        let gates_len = self.read_usize()?;
        let mut gates = Vec::with_capacity(gates_len);

        // We construct the common data without gates first,
        // to pass it as argument when reading the gates.
        let mut common_data = CommonCircuitData {
            config,
            fri_params,
            gates: vec![],
            selectors_info,
            quotient_degree_factor,
            num_gate_constraints,
            num_constants,
            num_public_inputs,
            k_is,
            num_partial_products,
            num_lookup_polys,
            num_lookup_selectors,
            luts,
        };

        for _ in 0..gates_len {
            let gate = self.read_gate::<F, D>(gate_serializer, &common_data)?;
            gates.push(gate);
        }

        common_data.gates = gates;

        Ok(common_data)
    }

    fn read_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<CircuitData<F, C, D>> {
        let common = self.read_common_circuit_data(gate_serializer)?;
        let prover_only = self.read_prover_only_circuit_data(generator_serializer, &common)?;
        let verifier_only = self.read_verifier_only_circuit_data()?;
        Ok(CircuitData {
            prover_only,
            verifier_only,
            common,
        })
    }

    fn read_prover_only_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<ProverOnlyCircuitData<F, C, D>> {
        let gen_len = self.read_usize()?;
        let mut generators = Vec::with_capacity(gen_len);
        for _ in 0..gen_len {
            generators.push(self.read_generator(generator_serializer, common_data)?);
        }
        let map_len = self.read_usize()?;
        let mut generator_indices_by_watches = BTreeMap::new();
        for _ in 0..map_len {
            let k = self.read_usize()?;
            generator_indices_by_watches.insert(k, self.read_usize_vec()?);
        }

        let constants_sigmas_commitment = self.read_polynomial_batch()?;
        let sigmas_len = self.read_usize()?;
        let mut sigmas = Vec::with_capacity(sigmas_len);
        for _ in 0..sigmas_len {
            let sigma_len = self.read_usize()?;
            sigmas.push(self.read_field_vec(sigma_len)?);
        }

        let subgroup_len = self.read_usize()?;
        let subgroup = self.read_field_vec(subgroup_len)?;

        let public_inputs = self.read_target_vec()?;

        let representative_map = self.read_usize_vec()?;

        let is_some = self.read_bool()?;
        let fft_root_table = match is_some {
            true => {
                let table_len = self.read_usize()?;
                let mut table = Vec::with_capacity(table_len);
                for _ in 0..table_len {
                    let len = self.read_usize()?;
                    table.push(self.read_field_vec(len)?);
                }
                Some(table)
            }
            false => None,
        };

        let circuit_digest = self.read_hash::<F, <C as GenericConfig<D>>::Hasher>()?;

        let length = self.read_usize()?;
        let mut lookup_rows = Vec::with_capacity(length);
        for _ in 0..length {
            lookup_rows.push(LookupWire {
                last_lu_gate: self.read_usize()?,
                last_lut_gate: self.read_usize()?,
                first_lut_gate: self.read_usize()?,
            });
        }

        let length = self.read_usize()?;
        let mut lut_to_lookups = Vec::with_capacity(length);
        for _ in 0..length {
            lut_to_lookups.push(self.read_target_lut()?);
        }

        Ok(ProverOnlyCircuitData {
            generators,
            generator_indices_by_watches,
            constants_sigmas_commitment,
            sigmas,
            subgroup,
            public_inputs,
            representative_map,
            fft_root_table,
            circuit_digest,
            lookup_rows,
            lut_to_lookups,
        })
    }

    fn read_prover_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<ProverCircuitData<F, C, D>> {
        let common = self.read_common_circuit_data(gate_serializer)?;
        let prover_only = self.read_prover_only_circuit_data(generator_serializer, &common)?;
        Ok(ProverCircuitData {
            prover_only,
            common,
        })
    }

    fn read_verifier_only_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
    ) -> IoResult<VerifierOnlyCircuitData<C, D>> {
        let height = self.read_usize()?;
        let constants_sigmas_cap = self.read_merkle_cap(height)?;
        let circuit_digest = self.read_hash::<F, <C as GenericConfig<D>>::Hasher>()?;
        Ok(VerifierOnlyCircuitData {
            constants_sigmas_cap,
            circuit_digest,
        })
    }

    fn read_verifier_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> IoResult<VerifierCircuitData<F, C, D>> {
        let verifier_only = self.read_verifier_only_circuit_data()?;
        let common = self.read_common_circuit_data(gate_serializer)?;
        Ok(VerifierCircuitData {
            verifier_only,
            common,
        })
    }

    fn read_target_verifier_circuit(&mut self) -> IoResult<VerifierCircuitTarget> {
        let constants_sigmas_cap = self.read_target_merkle_cap()?;
        let circuit_digest = self.read_target_hash()?;
        Ok(VerifierCircuitTarget {
            constants_sigmas_cap,
            circuit_digest,
        })
    }

    /// Reads a value of type [`Proof`] from `self` with `common_data`.
    #[inline]
    fn read_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<Proof<F, C, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let wires_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let plonk_zs_partial_products_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let quotient_polys_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let openings = self.read_opening_set::<F, C, D>(common_data)?;
        let opening_proof = self.read_fri_proof::<F, C, D>(common_data)?;
        Ok(Proof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    /// Reads a value of type [`ProofTarget`] from `self`.
    #[inline]
    fn read_target_proof<const D: usize>(&mut self) -> IoResult<ProofTarget<D>> {
        let wires_cap = self.read_target_merkle_cap()?;
        let plonk_zs_partial_products_cap = self.read_target_merkle_cap()?;
        let quotient_polys_cap = self.read_target_merkle_cap()?;
        let openings = self.read_target_opening_set::<D>()?;
        let opening_proof = self.read_target_fri_proof::<D>()?;
        Ok(ProofTarget {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    /// Reads a value of type [`ProofWithPublicInputs`] from `self` with `common_data`.
    #[inline]
    fn read_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<ProofWithPublicInputs<F, C, D>>
    where
        Self: Remaining,
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let proof = self.read_proof(common_data)?;
        let pi_len = self.read_usize()?;
        let public_inputs = self.read_field_vec(pi_len)?;
        Ok(ProofWithPublicInputs {
            proof,
            public_inputs,
        })
    }

    /// Reads a value of type [`ProofWithPublicInputsTarget`] from `self`.
    #[inline]
    fn read_target_proof_with_public_inputs<const D: usize>(
        &mut self,
    ) -> IoResult<ProofWithPublicInputsTarget<D>> {
        let proof = self.read_target_proof()?;
        let public_inputs = self.read_target_vec()?;
        Ok(ProofWithPublicInputsTarget {
            proof,
            public_inputs,
        })
    }

    /// Reads a value of type [`CompressedFriQueryRounds`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedFriQueryRounds<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let original_indices = (0..config.fri_config.num_query_rounds)
            .map(|_| self.read_u32().map(|i| i as usize))
            .collect::<Result<Vec<_>, _>>()?;
        let mut indices = original_indices.clone();
        indices.sort_unstable();
        indices.dedup();
        let mut pairs = Vec::new();
        for &i in &indices {
            pairs.push((i, self.read_fri_initial_proof::<F, C, D>(common_data)?));
        }
        let initial_trees_proofs = HashMap::from_iter(pairs);

        let mut steps = Vec::with_capacity(common_data.fri_params.reduction_arity_bits.len());
        for &a in &common_data.fri_params.reduction_arity_bits {
            indices.iter_mut().for_each(|x| {
                *x >>= a;
            });
            indices.dedup();
            let query_steps = (0..indices.len())
                .map(|_| self.read_fri_query_step::<F, C, D>(1 << a, true))
                .collect::<Result<Vec<_>, _>>()?;
            steps.push(
                indices
                    .iter()
                    .copied()
                    .zip(query_steps)
                    .collect::<HashMap<_, _>>(),
            );
        }

        Ok(CompressedFriQueryRounds {
            indices: original_indices,
            initial_trees_proofs,
            steps,
        })
    }

    /// Reads a value of type [`CompressedFriProof`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_fri_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedFriProof<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let commit_phase_merkle_caps = (0..common_data.fri_params.reduction_arity_bits.len())
            .map(|_| self.read_merkle_cap(config.fri_config.cap_height))
            .collect::<Result<Vec<_>, _>>()?;
        let query_round_proofs = self.read_compressed_fri_query_rounds::<F, C, D>(common_data)?;
        let final_poly = PolynomialCoeffs::new(
            self.read_field_ext_vec::<F, D>(common_data.fri_params.final_poly_len())?,
        );
        let pow_witness = self.read_field()?;
        Ok(CompressedFriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
        })
    }

    /// Reads a value of type [`CompressedProof`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedProof<F, C, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let wires_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let plonk_zs_partial_products_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let quotient_polys_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let openings = self.read_opening_set::<F, C, D>(common_data)?;
        let opening_proof = self.read_compressed_fri_proof::<F, C, D>(common_data)?;
        Ok(CompressedProof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    /// Reads a value of type [`CompressedProofWithPublicInputs`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedProofWithPublicInputs<F, C, D>>
    where
        Self: Remaining,
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let proof = self.read_compressed_proof(common_data)?;
        let public_inputs = self.read_field_vec(self.remaining() / size_of::<u64>())?;
        Ok(CompressedProofWithPublicInputs {
            proof,
            public_inputs,
        })
    }

    /// Reads a lookup table stored as `Vec<(u16, u16)>` from `self`.
    #[inline]
    fn read_lut(&mut self) -> IoResult<Vec<(u16, u16)>> {
        let length = self.read_usize()?;
        let mut lut = Vec::with_capacity(length);
        for _ in 0..length {
            lut.push((self.read_u16()?, self.read_u16()?));
        }

        Ok(lut)
    }

    /// Reads a target lookup table stored as `Lookup` from `self`.
    #[inline]
    fn read_target_lut(&mut self) -> IoResult<Lookup> {
        let length = self.read_usize()?;
        let mut lut = Vec::with_capacity(length);
        for _ in 0..length {
            lut.push((self.read_target()?, self.read_target()?));
        }

        Ok(lut)
    }
}

/// Writing
pub trait Write {
    /// Error Type
    type Error;

    /// Writes all `bytes` to `self`.
    fn write_all(&mut self, bytes: &[u8]) -> IoResult<()>;

    /// Writes a bool `x` to `self`.
    #[inline]
    fn write_bool(&mut self, x: bool) -> IoResult<()> {
        self.write_u8(u8::from(x))
    }

    /// Writes a target bool `x` to `self`.
    #[inline]
    fn write_target_bool(&mut self, x: BoolTarget) -> IoResult<()> {
        self.write_target(x.target)
    }

    /// Writes a vector of BoolTarget `v` to `self.`
    #[inline]
    fn write_target_bool_vec(&mut self, v: &[BoolTarget]) -> IoResult<()> {
        self.write_usize(v.len())?;
        for &elem in v.iter() {
            self.write_target_bool(elem)?;
        }

        Ok(())
    }

    /// Writes a byte `x` to `self`.
    #[inline]
    fn write_u8(&mut self, x: u8) -> IoResult<()> {
        self.write_all(&[x])
    }

    /// Writes a word `x` to `self`.
    #[inline]
    fn write_u16(&mut self, x: u16) -> IoResult<()> {
        self.write_all(&x.to_le_bytes())
    }

    /// Writes a word `x` to `self.`
    #[inline]
    fn write_u32(&mut self, x: u32) -> IoResult<()> {
        self.write_all(&x.to_le_bytes())
    }

    /// Writes a word `x` to `self.`
    #[inline]
    fn write_usize(&mut self, x: usize) -> IoResult<()> {
        self.write_all(&(x as u64).to_le_bytes())
    }

    /// Writes a vector of words `v` to `self.`
    #[inline]
    fn write_usize_vec(&mut self, v: &[usize]) -> IoResult<()> {
        self.write_usize(v.len())?;
        for &elem in v.iter() {
            self.write_usize(elem)?;
        }

        Ok(())
    }

    /// Writes an element `x` from the field `F` to `self`.
    #[inline]
    fn write_field<F>(&mut self, x: F) -> IoResult<()>
    where
        F: PrimeField64,
    {
        self.write_all(&x.to_canonical_u64().to_le_bytes())
    }

    /// Writes a vector `v` of elements from the field `F` to `self`.
    #[inline]
    fn write_field_vec<F>(&mut self, v: &[F]) -> IoResult<()>
    where
        F: PrimeField64,
    {
        for &a in v {
            self.write_field(a)?;
        }
        Ok(())
    }

    /// Writes an element `x` from the field extension of `F` to `self`.
    #[inline]
    fn write_field_ext<F, const D: usize>(&mut self, x: F::Extension) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
    {
        for &a in &x.to_basefield_array() {
            self.write_field(a)?;
        }
        Ok(())
    }

    /// Writes a vector `v` of elements from the field extension of `F` to `self`.
    #[inline]
    fn write_field_ext_vec<F, const D: usize>(&mut self, v: &[F::Extension]) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
    {
        for &a in v {
            self.write_field_ext::<F, D>(a)?;
        }
        Ok(())
    }

    /// Writes a Target `x` to `self.`
    #[inline]
    fn write_target(&mut self, x: Target) -> IoResult<()> {
        match x {
            Target::Wire(Wire { row, column }) => {
                self.write_bool(true)?;
                self.write_usize(row)?;
                self.write_usize(column)?;
            }
            Target::VirtualTarget { index } => {
                self.write_bool(false)?;
                self.write_usize(index)?;
            }
        };

        Ok(())
    }

    /// Writes an ExtensionTarget `x` to `self.`
    #[inline]
    fn write_target_ext<const D: usize>(&mut self, x: ExtensionTarget<D>) -> IoResult<()> {
        for &elem in x.0.iter() {
            self.write_target(elem)?;
        }

        Ok(())
    }

    /// Writes a vector of Target `v` to `self.`
    #[inline]
    fn write_target_array<const N: usize>(&mut self, v: &[Target; N]) -> IoResult<()> {
        for &elem in v.iter() {
            self.write_target(elem)?;
        }

        Ok(())
    }

    /// Writes a vector of Target `v` to `self.`
    #[inline]
    fn write_target_vec(&mut self, v: &[Target]) -> IoResult<()> {
        self.write_usize(v.len())?;
        for &elem in v.iter() {
            self.write_target(elem)?;
        }

        Ok(())
    }

    /// Writes a vector of ExtensionTarget `v` to `self.`
    #[inline]
    fn write_target_ext_vec<const D: usize>(&mut self, v: &[ExtensionTarget<D>]) -> IoResult<()> {
        self.write_usize(v.len())?;
        for &elem in v.iter() {
            self.write_target_ext(elem)?;
        }

        Ok(())
    }

    /// Writes a hash `h` to `self`.
    #[inline]
    fn write_hash<F, H>(&mut self, h: H::Hash) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        self.write_all(&h.to_bytes())
    }

    /// Writes a HashOutTarget `h` to `self`.
    #[inline]
    fn write_target_hash(&mut self, h: &HashOutTarget) -> IoResult<()> {
        for r in h.elements.iter() {
            self.write_target(*r)?;
        }

        Ok(())
    }

    /// Writes a vector of Hash `v` to `self.`
    #[inline]
    fn write_hash_vec<F, H>(&mut self, v: &[H::Hash]) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        self.write_usize(v.len())?;
        for &elem in v.iter() {
            self.write_hash::<F, H>(elem)?;
        }

        Ok(())
    }

    /// Writes `cap`, a value of type [`MerkleCap`], to `self`.
    #[inline]
    fn write_merkle_cap<F, H>(&mut self, cap: &MerkleCap<F, H>) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        for &a in &cap.0 {
            self.write_hash::<F, H>(a)?;
        }
        Ok(())
    }

    /// Writes `cap`, a value of type [`MerkleCapTarget`], to `self`.
    #[inline]
    fn write_target_merkle_cap(&mut self, cap: &MerkleCapTarget) -> IoResult<()> {
        self.write_usize(cap.0.len())?;
        for a in &cap.0 {
            self.write_target_hash(a)?;
        }
        Ok(())
    }

    /// Writes `tree`, a value of type [`MerkleTree`], to `self`.
    #[inline]
    fn write_merkle_tree<F, H>(&mut self, tree: &MerkleTree<F, H>) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        self.write_usize(tree.leaves.len())?;
        for i in 0..tree.leaves.len() {
            self.write_usize(tree.leaves[i].len())?;
            self.write_field_vec(&tree.leaves[i])?;
        }
        self.write_hash_vec::<F, H>(&tree.digests)?;
        self.write_usize(tree.cap.height())?;
        self.write_merkle_cap(&tree.cap)?;

        Ok(())
    }

    /// Writes a value `os` of type [`OpeningSet`] to `self.`
    #[inline]
    fn write_opening_set<F, const D: usize>(&mut self, os: &OpeningSet<F, D>) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
    {
        self.write_field_ext_vec::<F, D>(&os.constants)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_sigmas)?;
        self.write_field_ext_vec::<F, D>(&os.wires)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs_next)?;
        self.write_field_ext_vec::<F, D>(&os.lookup_zs)?;
        self.write_field_ext_vec::<F, D>(&os.lookup_zs_next)?;
        self.write_field_ext_vec::<F, D>(&os.partial_products)?;
        self.write_field_ext_vec::<F, D>(&os.quotient_polys)
    }

    /// Writes a value `os` of type [`OpeningSet`] to `self.`
    #[inline]
    fn write_target_opening_set<const D: usize>(
        &mut self,
        os: &OpeningSetTarget<D>,
    ) -> IoResult<()> {
        self.write_target_ext_vec::<D>(&os.constants)?;
        self.write_target_ext_vec::<D>(&os.plonk_sigmas)?;
        self.write_target_ext_vec::<D>(&os.wires)?;
        self.write_target_ext_vec::<D>(&os.plonk_zs)?;
        self.write_target_ext_vec::<D>(&os.plonk_zs_next)?;
        self.write_target_ext_vec::<D>(&os.lookup_zs)?;
        self.write_target_ext_vec::<D>(&os.next_lookup_zs)?;
        self.write_target_ext_vec::<D>(&os.partial_products)?;
        self.write_target_ext_vec::<D>(&os.quotient_polys)
    }

    /// Writes a value `p` of type [`MerkleProof`] to `self.`
    #[inline]
    fn write_merkle_proof<F, H>(&mut self, p: &MerkleProof<F, H>) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let length = p.siblings.len();
        self.write_u8(
            length
                .try_into()
                .expect("Merkle proof length must fit in u8."),
        )?;
        for &h in &p.siblings {
            self.write_hash::<F, H>(h)?;
        }
        Ok(())
    }

    /// Writes a value `pt` of type [`MerkleProofTarget`] to `self.`
    #[inline]
    fn write_target_merkle_proof(&mut self, pt: &MerkleProofTarget) -> IoResult<()> {
        let length = pt.siblings.len();
        self.write_u8(
            length
                .try_into()
                .expect("Merkle proof length must fit in u8."),
        )?;
        for h in &pt.siblings {
            self.write_target_hash(h)?;
        }
        Ok(())
    }

    /// Writes a value `fitp` of type [`FriInitialTreeProof`] to `self.`
    #[inline]
    fn write_fri_initial_proof<F, C, const D: usize>(
        &mut self,
        fitp: &FriInitialTreeProof<F, C::Hasher>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for (v, p) in &fitp.evals_proofs {
            self.write_field_vec(v)?;
            self.write_merkle_proof(p)?;
        }
        Ok(())
    }

    /// Writes a value `fitpt` of type [`FriInitialTreeProofTarget`] to `self.`
    #[inline]
    fn write_target_fri_initial_proof(
        &mut self,
        fitpt: &FriInitialTreeProofTarget,
    ) -> IoResult<()> {
        self.write_usize(fitpt.evals_proofs.len())?;
        for (v, p) in &fitpt.evals_proofs {
            self.write_target_vec(v)?;
            self.write_target_merkle_proof(p)?;
        }
        Ok(())
    }

    /// Writes a value `fqs` of type [`FriQueryStep`] to `self.`
    #[inline]
    fn write_fri_query_step<F, C, const D: usize>(
        &mut self,
        fqs: &FriQueryStep<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        self.write_field_ext_vec::<F, D>(&fqs.evals)?;
        self.write_merkle_proof(&fqs.merkle_proof)
    }

    /// Writes a value `fqst` of type [`FriQueryStepTarget`] to `self.`
    #[inline]
    fn write_target_fri_query_step<const D: usize>(
        &mut self,
        fqst: &FriQueryStepTarget<D>,
    ) -> IoResult<()> {
        self.write_target_ext_vec(&fqst.evals)?;
        self.write_target_merkle_proof(&fqst.merkle_proof)
    }

    /// Writes a value `fqrs` of type [`FriQueryRound`] to `self.`
    #[inline]
    fn write_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        fqrs: &[FriQueryRound<F, C::Hasher, D>],
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for fqr in fqrs {
            self.write_fri_initial_proof::<F, C, D>(&fqr.initial_trees_proof)?;
            for fqs in &fqr.steps {
                self.write_fri_query_step::<F, C, D>(fqs)?;
            }
        }
        Ok(())
    }

    /// Writes a value `fqrst` of type [`FriQueryRoundTarget`] to `self.`
    #[inline]
    fn write_target_fri_query_rounds<const D: usize>(
        &mut self,
        fqrst: &[FriQueryRoundTarget<D>],
    ) -> IoResult<()> {
        self.write_usize(fqrst.len())?;
        for fqr in fqrst {
            self.write_target_fri_initial_proof(&fqr.initial_trees_proof)?;
            self.write_usize(fqr.steps.len())?;
            for fqs in &fqr.steps {
                self.write_target_fri_query_step::<D>(fqs)?;
            }
        }
        Ok(())
    }

    /// Writes a value `fp` of type [`FriProof`] to `self.`
    #[inline]
    fn write_fri_proof<F, C, const D: usize>(
        &mut self,
        fp: &FriProof<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for cap in &fp.commit_phase_merkle_caps {
            self.write_merkle_cap(cap)?;
        }
        self.write_fri_query_rounds::<F, C, D>(&fp.query_round_proofs)?;
        self.write_field_ext_vec::<F, D>(&fp.final_poly.coeffs)?;
        self.write_field(fp.pow_witness)
    }

    /// Writes a value `fpt` of type [`FriProofTarget`] to `self.`
    #[inline]
    fn write_target_fri_proof<const D: usize>(&mut self, fpt: &FriProofTarget<D>) -> IoResult<()> {
        self.write_usize(fpt.commit_phase_merkle_caps.len())?;
        for cap in &fpt.commit_phase_merkle_caps {
            self.write_target_merkle_cap(cap)?;
        }
        self.write_target_fri_query_rounds::<D>(&fpt.query_round_proofs)?;
        self.write_target_ext_vec::<D>(&fpt.final_poly.0)?;
        self.write_target(fpt.pow_witness)
    }

    fn write_fri_reduction_strategy(
        &mut self,
        reduction_strategy: &FriReductionStrategy,
    ) -> IoResult<()> {
        match reduction_strategy {
            FriReductionStrategy::Fixed(seq) => {
                self.write_u8(0)?;
                self.write_usize_vec(seq.as_slice())?;

                Ok(())
            }
            FriReductionStrategy::ConstantArityBits(arity_bits, final_poly_bits) => {
                self.write_u8(1)?;
                self.write_usize(*arity_bits)?;
                self.write_usize(*final_poly_bits)?;

                Ok(())
            }
            FriReductionStrategy::MinSize(max) => {
                self.write_u8(2)?;
                if let Some(max) = max {
                    self.write_u8(1)?;
                    self.write_usize(*max)?;
                } else {
                    self.write_u8(0)?;
                }

                Ok(())
            }
        }
    }

    fn write_fri_config(&mut self, config: &FriConfig) -> IoResult<()> {
        let FriConfig {
            rate_bits,
            cap_height,
            num_query_rounds,
            proof_of_work_bits,
            reduction_strategy,
        } = &config;

        self.write_usize(*rate_bits)?;
        self.write_usize(*cap_height)?;
        self.write_usize(*num_query_rounds)?;
        self.write_u32(*proof_of_work_bits)?;
        self.write_fri_reduction_strategy(reduction_strategy)?;

        Ok(())
    }

    fn write_fri_params(&mut self, fri_params: &FriParams) -> IoResult<()> {
        let FriParams {
            config,
            reduction_arity_bits,
            degree_bits,
            hiding,
        } = fri_params;

        self.write_fri_config(config)?;
        self.write_usize_vec(reduction_arity_bits.as_slice())?;
        self.write_usize(*degree_bits)?;
        self.write_bool(*hiding)?;

        Ok(())
    }

    fn write_circuit_config(&mut self, config: &CircuitConfig) -> IoResult<()> {
        let CircuitConfig {
            num_wires,
            num_routed_wires,
            num_constants,
            security_bits,
            num_challenges,
            max_quotient_degree_factor,
            use_base_arithmetic_gate,
            zero_knowledge,
            fri_config,
        } = config;

        self.write_usize(*num_wires)?;
        self.write_usize(*num_routed_wires)?;
        self.write_usize(*num_constants)?;
        self.write_usize(*security_bits)?;
        self.write_usize(*num_challenges)?;
        self.write_usize(*max_quotient_degree_factor)?;
        self.write_bool(*use_base_arithmetic_gate)?;
        self.write_bool(*zero_knowledge)?;
        self.write_fri_config(fri_config)?;

        Ok(())
    }

    fn write_gate<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        gate: &GateRef<F, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()>;

    fn write_generator<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        generator: &WitnessGeneratorRef<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()>;

    fn write_selectors_info(&mut self, selectors_info: &SelectorsInfo) -> IoResult<()> {
        let SelectorsInfo {
            selector_indices,
            groups,
        } = selectors_info;

        self.write_usize_vec(selector_indices.as_slice())?;
        self.write_usize(groups.len())?;
        for group in groups.iter() {
            self.write_usize(group.start)?;
            self.write_usize(group.end)?;
        }
        Ok(())
    }

    fn write_polynomial_batch<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        poly_batch: &PolynomialBatch<F, C, D>,
    ) -> IoResult<()> {
        self.write_usize(poly_batch.polynomials.len())?;
        for i in 0..poly_batch.polynomials.len() {
            self.write_usize(poly_batch.polynomials[i].coeffs.len())?;
            self.write_field_vec(&poly_batch.polynomials[i].coeffs)?;
        }
        self.write_merkle_tree(&poly_batch.merkle_tree)?;
        self.write_usize(poly_batch.degree_log)?;
        self.write_usize(poly_batch.rate_bits)?;
        self.write_bool(poly_batch.blinding)?;

        Ok(())
    }

    fn write_common_circuit_data<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> IoResult<()> {
        let CommonCircuitData {
            config,
            fri_params,
            gates,
            selectors_info,
            quotient_degree_factor,
            num_gate_constraints,
            num_constants,
            num_public_inputs,
            k_is,
            num_partial_products,
            num_lookup_polys,
            num_lookup_selectors,
            luts,
        } = common_data;

        self.write_circuit_config(config)?;
        self.write_fri_params(fri_params)?;

        self.write_selectors_info(selectors_info)?;
        self.write_usize(*quotient_degree_factor)?;
        self.write_usize(*num_gate_constraints)?;
        self.write_usize(*num_constants)?;
        self.write_usize(*num_public_inputs)?;

        self.write_usize(k_is.len())?;
        self.write_field_vec(k_is.as_slice())?;

        self.write_usize(*num_partial_products)?;

        self.write_usize(*num_lookup_polys)?;
        self.write_usize(*num_lookup_selectors)?;
        self.write_usize(luts.len())?;
        for lut in luts.iter() {
            self.write_lut(lut)?;
        }

        self.write_usize(gates.len())?;
        for gate in gates.iter() {
            self.write_gate::<F, D>(gate, gate_serializer, common_data)?;
        }

        Ok(())
    }

    fn write_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        circuit_data: &CircuitData<F, C, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<()> {
        self.write_common_circuit_data(&circuit_data.common, gate_serializer)?;
        self.write_prover_only_circuit_data(
            &circuit_data.prover_only,
            generator_serializer,
            &circuit_data.common,
        )?;
        self.write_verifier_only_circuit_data(&circuit_data.verifier_only)
    }

    fn write_prover_only_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        prover_only_circuit_data: &ProverOnlyCircuitData<F, C, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()> {
        let ProverOnlyCircuitData {
            generators,
            generator_indices_by_watches,
            constants_sigmas_commitment,
            sigmas,
            subgroup,
            public_inputs,
            representative_map,
            fft_root_table,
            circuit_digest,
            lookup_rows,
            lut_to_lookups,
        } = prover_only_circuit_data;

        self.write_usize(generators.len())?;
        for generator in generators.iter() {
            self.write_generator::<F, D>(generator, generator_serializer, common_data)?;
        }

        self.write_usize(generator_indices_by_watches.len())?;
        for (k, v) in generator_indices_by_watches {
            self.write_usize(*k)?;
            self.write_usize_vec(v)?;
        }

        self.write_polynomial_batch(constants_sigmas_commitment)?;
        self.write_usize(sigmas.len())?;
        for i in 0..sigmas.len() {
            self.write_usize(sigmas[i].len())?;
            self.write_field_vec(&sigmas[i])?;
        }
        self.write_usize(subgroup.len())?;
        self.write_field_vec(subgroup)?;
        self.write_target_vec(public_inputs)?;
        self.write_usize_vec(representative_map)?;

        match fft_root_table {
            Some(table) => {
                self.write_bool(true)?;
                self.write_usize(table.len())?;
                for i in 0..table.len() {
                    self.write_usize(table[i].len())?;
                    self.write_field_vec(&table[i])?;
                }
            }
            None => self.write_bool(false)?,
        }

        self.write_hash::<F, <C as GenericConfig<D>>::Hasher>(*circuit_digest)?;

        self.write_usize(lookup_rows.len())?;
        for wire in lookup_rows.iter() {
            self.write_usize(wire.last_lu_gate)?;
            self.write_usize(wire.last_lut_gate)?;
            self.write_usize(wire.first_lut_gate)?;
        }

        self.write_usize(lut_to_lookups.len())?;
        for tlut in lut_to_lookups.iter() {
            self.write_target_lut(tlut)?;
        }

        Ok(())
    }

    fn write_prover_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        prover_circuit_data: &ProverCircuitData<F, C, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
    ) -> IoResult<()> {
        self.write_common_circuit_data(&prover_circuit_data.common, gate_serializer)?;
        self.write_prover_only_circuit_data(
            &prover_circuit_data.prover_only,
            generator_serializer,
            &prover_circuit_data.common,
        )
    }

    fn write_verifier_only_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        verifier_only_circuit_data: &VerifierOnlyCircuitData<C, D>,
    ) -> IoResult<()> {
        let VerifierOnlyCircuitData {
            constants_sigmas_cap,
            circuit_digest,
        } = verifier_only_circuit_data;

        self.write_usize(constants_sigmas_cap.height())?;
        self.write_merkle_cap(constants_sigmas_cap)?;
        self.write_hash::<F, <C as GenericConfig<D>>::Hasher>(*circuit_digest)?;

        Ok(())
    }

    fn write_verifier_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        verifier_circuit_data: &VerifierCircuitData<F, C, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> IoResult<()> {
        self.write_verifier_only_circuit_data(&verifier_circuit_data.verifier_only)?;
        self.write_common_circuit_data(&verifier_circuit_data.common, gate_serializer)
    }

    fn write_target_verifier_circuit(
        &mut self,
        verifier_circuit: &VerifierCircuitTarget,
    ) -> IoResult<()> {
        let VerifierCircuitTarget {
            constants_sigmas_cap,
            circuit_digest,
        } = verifier_circuit;

        self.write_target_merkle_cap(constants_sigmas_cap)?;
        self.write_target_hash(circuit_digest)?;

        Ok(())
    }

    /// Writes a value `proof` of type [`Proof`] to `self.`
    #[inline]
    fn write_proof<F, C, const D: usize>(&mut self, proof: &Proof<F, C, D>) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        self.write_merkle_cap(&proof.wires_cap)?;
        self.write_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_opening_set(&proof.openings)?;
        self.write_fri_proof::<F, C, D>(&proof.opening_proof)
    }

    /// Writes a value `proof` of type [`Proof`] to `self.`
    #[inline]
    fn write_target_proof<const D: usize>(&mut self, proof: &ProofTarget<D>) -> IoResult<()> {
        self.write_target_merkle_cap(&proof.wires_cap)?;
        self.write_target_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_target_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_target_opening_set(&proof.openings)?;
        self.write_target_fri_proof::<D>(&proof.opening_proof)
    }

    /// Writes a value `proof_with_pis` of type [`ProofWithPublicInputs`] to `self.`
    #[inline]
    fn write_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        proof_with_pis: &ProofWithPublicInputs<F, C, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let ProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof_with_pis;
        self.write_proof(proof)?;
        self.write_usize(public_inputs.len())?;
        self.write_field_vec(public_inputs)
    }

    /// Writes a value `proof_with_pis` of type [`ProofWithPublicInputsTarget`] to `self.`
    #[inline]
    fn write_target_proof_with_public_inputs<const D: usize>(
        &mut self,
        proof_with_pis: &ProofWithPublicInputsTarget<D>,
    ) -> IoResult<()> {
        let ProofWithPublicInputsTarget {
            proof,
            public_inputs,
        } = proof_with_pis;
        self.write_target_proof(proof)?;
        self.write_target_vec(public_inputs)
    }

    /// Writes a value `cfqrs` of type [`CompressedFriQueryRounds`] to `self.`
    #[inline]
    fn write_compressed_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        cfqrs: &CompressedFriQueryRounds<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for &i in &cfqrs.indices {
            self.write_u32(i as u32)?;
        }
        let mut initial_trees_proofs = cfqrs.initial_trees_proofs.iter().collect::<Vec<_>>();
        initial_trees_proofs.sort_by_key(|&x| x.0);
        for (_, itp) in initial_trees_proofs {
            self.write_fri_initial_proof::<F, C, D>(itp)?;
        }
        for h in &cfqrs.steps {
            let mut fri_query_steps = h.iter().collect::<Vec<_>>();
            fri_query_steps.sort_by_key(|&x| x.0);
            for (_, fqs) in fri_query_steps {
                self.write_fri_query_step::<F, C, D>(fqs)?;
            }
        }
        Ok(())
    }

    /// Writes a value `fq` of type [`CompressedFriProof`] to `self.`
    #[inline]
    fn write_compressed_fri_proof<F, C, const D: usize>(
        &mut self,
        fp: &CompressedFriProof<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for cap in &fp.commit_phase_merkle_caps {
            self.write_merkle_cap(cap)?;
        }
        self.write_compressed_fri_query_rounds::<F, C, D>(&fp.query_round_proofs)?;
        self.write_field_ext_vec::<F, D>(&fp.final_poly.coeffs)?;
        self.write_field(fp.pow_witness)
    }

    /// Writes a value `proof` of type [`CompressedProof`] to `self.`
    #[inline]
    fn write_compressed_proof<F, C, const D: usize>(
        &mut self,
        proof: &CompressedProof<F, C, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        self.write_merkle_cap(&proof.wires_cap)?;
        self.write_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_opening_set(&proof.openings)?;
        self.write_compressed_fri_proof::<F, C, D>(&proof.opening_proof)
    }

    /// Writes a value `proof_with_pis` of type [`CompressedProofWithPublicInputs`] to `self.`
    #[inline]
    fn write_compressed_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        proof_with_pis: &CompressedProofWithPublicInputs<F, C, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let CompressedProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof_with_pis;
        self.write_compressed_proof(proof)?;
        self.write_field_vec(public_inputs)
    }

    /// Writes a lookup table to `self`.
    #[inline]
    fn write_lut(&mut self, lut: &[(u16, u16)]) -> IoResult<()> {
        self.write_usize(lut.len())?;
        for (a, b) in lut.iter() {
            self.write_u16(*a)?;
            self.write_u16(*b)?;
        }

        Ok(())
    }

    /// Writes a target lookup table to `self`.
    #[inline]
    fn write_target_lut(&mut self, lut: &[(Target, Target)]) -> IoResult<()> {
        self.write_usize(lut.len())?;
        for (a, b) in lut.iter() {
            self.write_target(*a)?;
            self.write_target(*b)?;
        }

        Ok(())
    }
}

impl Write for Vec<u8> {
    type Error = Infallible;

    #[inline]
    fn write_all(&mut self, bytes: &[u8]) -> IoResult<()> {
        self.extend_from_slice(bytes);
        Ok(())
    }

    fn write_gate<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        gate: &GateRef<F, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()> {
        gate_serializer.write_gate(self, gate, common_data)
    }

    fn write_generator<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        generator: &WitnessGeneratorRef<F, D>,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()> {
        generator_serializer.write_generator(self, generator, common_data)
    }
}

/// Buffer
#[derive(Debug)]
pub struct Buffer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Buffer<'a> {
    /// Builds a new [`Buffer`] over `buffer`.
    #[inline]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Returns the inner position.
    #[inline]
    pub const fn pos(&self) -> usize {
        self.pos
    }

    /// Returns the inner buffer.
    #[inline]
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Returns the inner unread buffer.
    #[inline]
    pub fn unread_bytes(&self) -> &'a [u8] {
        &self.bytes()[self.pos()..]
    }
}

impl Remaining for Buffer<'_> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos()
    }
}

impl Read for Buffer<'_> {
    #[inline]
    fn read_exact(&mut self, bytes: &mut [u8]) -> IoResult<()> {
        let n = bytes.len();
        if self.remaining() < n {
            Err(IoError)
        } else {
            bytes.copy_from_slice(&self.bytes[self.pos..][..n]);
            self.pos += n;
            Ok(())
        }
    }

    fn read_gate<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<GateRef<F, D>> {
        gate_serializer.read_gate(self, common_data)
    }

    fn read_generator<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        generator_serializer: &dyn WitnessGeneratorSerializer<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<WitnessGeneratorRef<F, D>> {
        generator_serializer.read_generator(self, common_data)
    }
}
