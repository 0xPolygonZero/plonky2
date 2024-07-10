//! All the different proof types and their associated `circuit` versions
//! to be used when proving (recursive) [`Stark`][crate::stark::Stark]
//! statements

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use core::ops::Range;

use plonky2::batch_fri::oracle::BatchFriOracle;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::fri::proof::{CompressedFriProof, FriProof, FriProofTarget};
use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::target::Target;
use plonky2::plonk::config::{GenericConfig, Hasher};
use plonky2::util::serialization::{Buffer, IoResult, Read, Write};
use plonky2_maybe_rayon::*;

use crate::config::StarkConfig;
use crate::lookup::GrandProductChallengeSet;
use crate::proof::{StarkOpeningSet, StarkOpeningSetTarget};
use crate::stark::Stark;

/// Merkle caps and openings that form the proof of multiple STARKs.
#[derive(Debug, Clone)]
pub struct BatchStarkProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N: usize,
> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Optional merkle cap of LDEs of permutation Z values, if any.
    pub auxiliary_polys_cap: Option<MerkleCap<F, C::Hasher>>,
    /// Merkle cap of LDEs of trace values.
    pub quotient_polys_cap: Option<MerkleCap<F, C::Hasher>>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: [StarkOpeningSet<F, D>; N],
    /// A batch FRI argument for all openings.
    pub opening_proof: FriProof<F, C::Hasher, D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize, const N: usize>
    BatchStarkProof<F, C, D, N>
{
    /// Recover the length of the trace from a batched STARK proof and a STARK config.
    pub fn recover_degree_bits(&self, config: &StarkConfig) -> usize {
        let initial_merkle_proof = &self.opening_proof.query_round_proofs[0]
            .initial_trees_proof
            .evals_proofs[0]
            .1;
        let lde_bits = config.fri_config.cap_height + initial_merkle_proof.siblings.len();
        lde_bits - config.fri_config.rate_bits
    }
}

/// Circuit version of [`BatchStarkProof`].
/// Merkle caps and openings that form the proof of multiple STARKs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchStarkProofTarget<const D: usize, const N: usize> {
    /// `Target` for the Merkle cap trace values LDEs.
    pub trace_cap: MerkleCapTarget,
    /// Optional `Target` for the Merkle cap of lookup helper and CTL columns LDEs, if any.
    pub auxiliary_polys_cap: Option<MerkleCapTarget>,
    /// `Target` for the Merkle cap of quotient polynomial evaluations LDEs.
    pub quotient_polys_cap: Option<MerkleCapTarget>,
    /// `Target`s for the purported values of each polynomial at the challenge point. One opening set per STARK.
    pub openings: [StarkOpeningSetTarget<D>; N],
    /// `Target`s for the batch FRI argument for all openings.
    pub opening_proof: FriProofTarget<D>,
}

impl<const D: usize, const N: usize> BatchStarkProofTarget<D, N> {
    /// Serializes a batched STARK proof.
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target_merkle_cap(&self.trace_cap)?;
        buffer.write_bool(self.auxiliary_polys_cap.is_some())?;
        if let Some(poly) = &self.auxiliary_polys_cap {
            buffer.write_target_merkle_cap(poly)?;
        }
        buffer.write_bool(self.quotient_polys_cap.is_some())?;
        if let Some(poly) = &self.quotient_polys_cap {
            buffer.write_target_merkle_cap(poly)?;
        }
        buffer.write_target_fri_proof(&self.opening_proof)?;
        for opening_set in self.openings.iter() {
            opening_set.to_buffer(buffer)?;
        }
        Ok(())
    }

    /// Deserializes a batched STARK proof.
    pub fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let trace_cap = buffer.read_target_merkle_cap()?;
        let auxiliary_polys_cap = if buffer.read_bool()? {
            Some(buffer.read_target_merkle_cap()?)
        } else {
            None
        };
        let quotient_polys_cap = if buffer.read_bool()? {
            Some(buffer.read_target_merkle_cap()?)
        } else {
            None
        };
        let opening_proof = buffer.read_target_fri_proof()?;
        let mut openings = Vec::new();
        for _ in 0..N {
            openings.push(StarkOpeningSetTarget::from_buffer(buffer)?);
        }

        Ok(Self {
            trace_cap,
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings: openings.try_into().unwrap(),
            opening_proof,
        })
    }

    /// Recover the length of the trace from a STARK proof and a STARK config.
    pub fn recover_degree_bits(&self, config: &StarkConfig) -> usize {
        let initial_merkle_proof = &self.opening_proof.query_round_proofs[0]
            .initial_trees_proof
            .evals_proofs[0]
            .1;
        let lde_bits = config.fri_config.cap_height + initial_merkle_proof.siblings.len();
        lde_bits - config.fri_config.rate_bits
    }
}

/// Merkle caps and openings that form the proof of multiple STARKs, along with its public inputs.
#[derive(Debug, Clone)]
pub struct BatchStarkProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N: usize,
> {
    /// A batched STARK proof.
    pub proof: BatchStarkProof<F, C, D, N>,
    /// Public inputs associated to this STARK proof.
    // TODO: Maybe make it generic over a `S: Stark` and replace with `[F; S::PUBLIC_INPUTS]`.
    pub public_inputs: Vec<F>,
}

/// Circuit version of [BatchStarkProofWithPublicInputs`].
#[derive(Debug, Clone)]
pub struct BatchStarkProofWithPublicInputsTarget<const D: usize, const N: usize> {
    /// `Target` STARK proof.
    pub proof: BatchStarkProofTarget<D, N>,
    /// `Target` public inputs for this STARK proof.
    pub public_inputs: Vec<Target>,
}

/// A compressed proof format of multiple STARKs.
#[derive(Debug, Clone)]
pub struct CompressedBatchStarkProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N: usize,
> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: [StarkOpeningSet<F, D>; N],
    /// A batch FRI argument for all openings.
    pub opening_proof: CompressedFriProof<F, C::Hasher, D>,
}

/// A compressed [`BatchStarkProof`] format of multiple STARKs with its public inputs.
#[derive(Debug, Clone)]
pub struct CompressedBatchStarkProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N: usize,
> {
    /// A compressed btached STARK proof.
    pub proof: CompressedBatchStarkProof<F, C, D, N>,
    /// Public inputs for this compressed STARK proof.
    pub public_inputs: Vec<F>,
}

/// A [`BatchStarkProof`] along with metadata about the initial Fiat-Shamir state, which is used when
/// creating a recursive wrapper proof around a STARK proof.
#[derive(Debug, Clone)]
pub struct BatchStarkProofWithMetadata<F, C, const D: usize, const N: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Initial Fiat-Shamir state.
    pub init_challenger_state: <C::Hasher as Hasher<F>>::Permutation,
    /// Proof for multiple STARKs.
    pub proof: BatchStarkProof<F, C, D, N>,
}

/// A combination of a batched STARK proof for independent statements operating on possibly shared variables,
/// along with Cross-Table Lookup (CTL) challenges to assert consistency of common variables across tables.
#[derive(Debug, Clone)]
pub struct BatchMultiProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N: usize,
> {
    /// Proof for all the different STARK modules.
    pub stark_proofs: BatchStarkProofWithMetadata<F, C, D, N>,
    /// Cross-table lookup challenges.
    pub ctl_challenges: GrandProductChallengeSet<F>,
}

// impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize, const N: usize>
//     MultiProof<F, C, D, N>
// {
//     /// Returns the degree (i.e. the trace length) of each STARK proof,
//     /// from their common [`StarkConfig`].
//     pub fn recover_degree_bits(&self, config: &StarkConfig) -> [usize; N] {
//         core::array::from_fn(|i| self.stark_proofs[i].proof.recover_degree_bits(config))
//     }
// }

impl<F: RichField + Extendable<D>, const D: usize> StarkOpeningSet<F, D> {
    /// Returns a `StarkOpeningSet` given a STARK, the batched polynomial commitments, the evaluation point and a generator `g`.
    ///
    /// Polynomials are evaluated at point `zeta` and, if necessary, at `g * zeta`.
    pub fn new_from_batch<C: GenericConfig<D, F = F>, S: Stark<F, D>>(
        stark: S,
        zeta: F::Extension,
        g: F,
        trace_commitment: &BatchFriOracle<F, C, D>,
        trace_polys_range: Range<usize>,
        auxiliary_polys_commitment: &BatchFriOracle<F, C, D>,
        auxiliary_polys_range: Range<usize>,
        quotient_commitment: &BatchFriOracle<F, C, D>,
        quotient_polys_range: Range<usize>,
        num_lookup_columns: usize,
        num_ctl_polys: &[usize],
    ) -> Self {
        // Batch evaluates polynomials on the LDE, at a point `z`.
        let eval_commitment =
            |z: F::Extension, c: &BatchFriOracle<F, C, D>, range: Range<usize>| {
                c.polynomials[range]
                    .par_iter()
                    .map(|p| p.to_extension().eval(z))
                    .collect::<Vec<_>>()
            };
        // Batch evaluates polynomials at a base field point `z`.
        let eval_commitment_base = |z: F, c: &BatchFriOracle<F, C, D>, range: Range<usize>| {
            c.polynomials[range]
                .par_iter()
                .map(|p| p.eval(z))
                .collect::<Vec<_>>()
        };

        let auxiliary_first = eval_commitment_base(
            F::ONE,
            auxiliary_polys_commitment,
            auxiliary_polys_range.clone(),
        );
        // `g * zeta`.
        let zeta_next = zeta.scalar_mul(g);

        Self {
            local_values: eval_commitment(zeta, trace_commitment, trace_polys_range.clone()),
            next_values: eval_commitment(zeta_next, trace_commitment, trace_polys_range),
            auxiliary_polys: Some(eval_commitment(
                zeta,
                auxiliary_polys_commitment,
                auxiliary_polys_range.clone(),
            )),
            auxiliary_polys_next: Some(eval_commitment(
                zeta_next,
                auxiliary_polys_commitment,
                auxiliary_polys_range,
            )),
            ctl_zs_first: stark.requires_ctls().then(|| {
                let total_num_helper_cols: usize = num_ctl_polys.iter().sum();
                auxiliary_first[num_lookup_columns + total_num_helper_cols..].to_vec()
            }),
            quotient_polys: Some(eval_commitment(
                zeta,
                quotient_commitment,
                quotient_polys_range,
            )),
        }
    }
}
