use alloc::vec;
use alloc::vec::Vec;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::fri::proof::{
    CompressedFriProof, FriChallenges, FriChallengesTarget, FriProof, FriProofTarget,
};
use plonky2::fri::structure::{
    FriOpeningBatch, FriOpeningBatchTarget, FriOpenings, FriOpeningsTarget,
};
use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::config::GenericConfig;
use plonky2_maybe_rayon::*;

use crate::config::StarkConfig;
use crate::lookup::GrandProductChallengeSet;

#[derive(Debug, Clone)]
pub struct StarkProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of permutation Z values.
    pub auxiliary_polys_cap: Option<MerkleCap<F, C::Hasher>>,
    /// Merkle cap of LDEs of trace values.
    pub quotient_polys_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: FriProof<F, C::Hasher, D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> StarkProof<F, C, D> {
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

pub struct StarkProofTarget<const D: usize> {
    pub trace_cap: MerkleCapTarget,
    pub auxiliary_polys_cap: Option<MerkleCapTarget>,
    pub quotient_polys_cap: MerkleCapTarget,
    pub openings: StarkOpeningSetTarget<D>,
    pub opening_proof: FriProofTarget<D>,
}

impl<const D: usize> StarkProofTarget<D> {
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

#[derive(Debug, Clone)]
pub struct StarkProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof: StarkProof<F, C, D>,
    // TODO: Maybe make it generic over a `S: Stark` and replace with `[F; S::PUBLIC_INPUTS]`.
    pub public_inputs: Vec<F>,
}

pub struct StarkProofWithPublicInputsTarget<const D: usize> {
    pub proof: StarkProofTarget<D>,
    pub public_inputs: Vec<Target>,
}

pub struct CompressedStarkProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: CompressedFriProof<F, C::Hasher, D>,
}

pub struct CompressedStarkProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof: CompressedStarkProof<F, C, D>,
    pub public_inputs: Vec<F>,
}

/// A combination of STARK proofs for independent statements operating on possibly shared variables,
/// along with Cross-Table Lookup (CTL) challenges to assert consistency of common variables across tables.
#[derive(Debug, Clone)]
pub struct MultiProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    const N: usize,
> {
    /// Proofs for all the different STARK modules.
    pub stark_proofs: [StarkProof<F, C, D>; N],
    /// Cross-table lookup challenges.
    pub ctl_challenges: GrandProductChallengeSet<F>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize, const N: usize>
    MultiProof<F, C, D, N>
{
    /// Returns the degree (i.e. the trace length) of each STARK proofs,
    /// from their common [`StarkConfig`].
    pub fn recover_degree_bits(&self, config: &StarkConfig) -> [usize; N] {
        core::array::from_fn(|i| self.stark_proofs[i].recover_degree_bits(config))
    }
}

pub struct StarkProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    /// Randomness used in any permutation arguments.
    pub lookup_challenge_set: Option<GrandProductChallengeSet<F>>,

    /// Random values used to combine STARK constraints.
    pub stark_alphas: Vec<F>,

    /// Point at which the STARK polynomials are opened.
    pub stark_zeta: F::Extension,

    pub fri_challenges: FriChallenges<F, D>,
}

pub(crate) struct StarkProofChallengesTarget<const D: usize> {
    pub lookup_challenge_set: Option<GrandProductChallengeSet<Target>>,
    pub stark_alphas: Vec<Target>,
    pub stark_zeta: ExtensionTarget<D>,
    pub fri_challenges: FriChallengesTarget<D>,
}

/// Randomness for all STARK proofs contained in a [`MultiProof`]`.
pub struct MultiProofChallenges<F: RichField + Extendable<D>, const D: usize, const N: usize> {
    /// Randomness used in each STARK proof.
    pub stark_challenges: [StarkProofChallenges<F, D>; N],
    /// Randomness used for cross-table lookups. It is shared by all STARKs.
    pub ctl_challenges: GrandProductChallengeSet<F>,
}

/// Purported values of each polynomial at the challenge point.
#[derive(Debug, Clone)]
pub struct StarkOpeningSet<F: RichField + Extendable<D>, const D: usize> {
    pub local_values: Vec<F::Extension>,
    pub next_values: Vec<F::Extension>,
    pub auxiliary_polys: Option<Vec<F::Extension>>,
    pub auxiliary_polys_next: Option<Vec<F::Extension>>,
    /// Openings of cross-table lookups `Z` polynomials at `1`.
    pub ctl_zs_first: Option<Vec<F>>,
    pub quotient_polys: Vec<F::Extension>,
}

impl<F: RichField + Extendable<D>, const D: usize> StarkOpeningSet<F, D> {
    pub fn new<C: GenericConfig<D, F = F>>(
        zeta: F::Extension,
        g: F,
        trace_commitment: &PolynomialBatch<F, C, D>,
        auxiliary_polys_commitment: Option<&PolynomialBatch<F, C, D>>,
        quotient_commitment: &PolynomialBatch<F, C, D>,
        num_lookup_columns: usize,
        num_ctl_polys: &[usize],
    ) -> Self {
        let total_num_helper_cols: usize = num_ctl_polys.iter().sum();

        // Batch evaluates polynomials on the LDE, at a point `z`.
        let eval_commitment = |z: F::Extension, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        // Batch evaluates polynomials at a base field point `z`.
        let eval_commitment_base = |z: F, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.eval(z))
                .collect::<Vec<_>>()
        };

        let auxiliary_first = auxiliary_polys_commitment.map(|c| eval_commitment_base(F::ONE, c));

        let zeta_next = zeta.scalar_mul(g);
        Self {
            local_values: eval_commitment(zeta, trace_commitment),
            next_values: eval_commitment(zeta_next, trace_commitment),
            auxiliary_polys: auxiliary_polys_commitment.map(|c| eval_commitment(zeta, c)),
            auxiliary_polys_next: auxiliary_polys_commitment.map(|c| eval_commitment(zeta_next, c)),
            ctl_zs_first: (total_num_helper_cols != 0).then(|| {
                auxiliary_first.unwrap()[num_lookup_columns + total_num_helper_cols..].to_vec()
            }),
            quotient_polys: eval_commitment(zeta, quotient_commitment),
        }
    }

    pub(crate) fn to_fri_openings(&self) -> FriOpenings<F, D> {
        let zeta_batch = FriOpeningBatch {
            values: self
                .local_values
                .iter()
                .chain(self.auxiliary_polys.iter().flatten())
                .chain(&self.quotient_polys)
                .copied()
                .collect_vec(),
        };
        let zeta_next_batch = FriOpeningBatch {
            values: self
                .next_values
                .iter()
                .chain(self.auxiliary_polys_next.iter().flatten())
                .copied()
                .collect_vec(),
        };

        if let Some(ctl_zs_first) = self.ctl_zs_first.as_ref() {
            debug_assert!(!ctl_zs_first.is_empty());
            debug_assert!(self.auxiliary_polys.is_some());
            debug_assert!(self.auxiliary_polys_next.is_some());

            let ctl_first_batch = FriOpeningBatch {
                values: ctl_zs_first
                    .iter()
                    .copied()
                    .map(F::Extension::from_basefield)
                    .collect(),
            };

            FriOpenings {
                batches: vec![zeta_batch, zeta_next_batch, ctl_first_batch],
            }
        } else {
            FriOpenings {
                batches: vec![zeta_batch, zeta_next_batch],
            }
        }
    }
}

pub struct StarkOpeningSetTarget<const D: usize> {
    pub local_values: Vec<ExtensionTarget<D>>,
    pub next_values: Vec<ExtensionTarget<D>>,
    pub auxiliary_polys: Option<Vec<ExtensionTarget<D>>>,
    pub auxiliary_polys_next: Option<Vec<ExtensionTarget<D>>>,
    pub ctl_zs_first: Option<Vec<Target>>,
    pub quotient_polys: Vec<ExtensionTarget<D>>,
}

impl<const D: usize> StarkOpeningSetTarget<D> {
    pub(crate) fn to_fri_openings(&self, zero: Target) -> FriOpeningsTarget<D> {
        let zeta_batch = FriOpeningBatchTarget {
            values: self
                .local_values
                .iter()
                .chain(self.auxiliary_polys.iter().flatten())
                .chain(&self.quotient_polys)
                .copied()
                .collect_vec(),
        };
        let zeta_next_batch = FriOpeningBatchTarget {
            values: self
                .next_values
                .iter()
                .chain(self.auxiliary_polys_next.iter().flatten())
                .copied()
                .collect_vec(),
        };

        if let Some(ctl_zs_first) = self.ctl_zs_first.as_ref() {
            debug_assert!(!ctl_zs_first.is_empty());
            debug_assert!(self.auxiliary_polys.is_some());
            debug_assert!(self.auxiliary_polys_next.is_some());

            let ctl_first_batch = FriOpeningBatchTarget {
                values: ctl_zs_first
                    .iter()
                    .copied()
                    .map(|t| t.to_ext_target(zero))
                    .collect(),
            };

            FriOpeningsTarget {
                batches: vec![zeta_batch, zeta_next_batch, ctl_first_batch],
            }
        } else {
            FriOpeningsTarget {
                batches: vec![zeta_batch, zeta_next_batch],
            }
        }
    }
}
