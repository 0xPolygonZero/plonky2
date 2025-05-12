//! All the different proof types and their associated `circuit` versions
//! to be used when proving (recursive) [`Stark`][crate::stark::Stark]
//! statements

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::fri::proof::{FriChallenges, FriChallengesTarget, FriProof, FriProofTarget};
use plonky2::fri::structure::{
    FriOpeningBatch, FriOpeningBatchTarget, FriOpenings, FriOpeningsTarget,
};
use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::config::{GenericConfig, Hasher};
use plonky2::util::serialization::{Buffer, IoResult, Read, Write};
use plonky2_maybe_rayon::*;
use serde::{Deserialize, Serialize};

use crate::config::StarkConfig;
use crate::lookup::GrandProductChallengeSet;

/// Merkle caps and openings that form the proof of a single STARK.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(bound = "")]
pub struct StarkProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Optional merkle cap of LDEs of permutation Z values, if any.
    pub auxiliary_polys_cap: Option<MerkleCap<F, C::Hasher>>,
    /// Merkle cap of LDEs of trace values.
    pub quotient_polys_cap: Option<MerkleCap<F, C::Hasher>>,
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

/// Circuit version of [`StarkProof`].
/// Merkle caps and openings that form the proof of a single STARK.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StarkProofTarget<const D: usize> {
    /// `Target` for the Merkle cap trace values LDEs.
    pub trace_cap: MerkleCapTarget,
    /// Optional `Target` for the Merkle cap of lookup helper and CTL columns LDEs, if any.
    pub auxiliary_polys_cap: Option<MerkleCapTarget>,
    /// `Target` for the Merkle cap of quotient polynomial evaluations LDEs.
    pub quotient_polys_cap: Option<MerkleCapTarget>,
    /// `Target`s for the purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSetTarget<D>,
    /// `Target`s for the batch FRI argument for all openings.
    pub opening_proof: FriProofTarget<D>,
    /// `Target`s for the proof's degree bits.
    pub degree_bits: Target,
}

impl<const D: usize> StarkProofTarget<D> {
    /// Serializes a STARK proof.
    pub fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target(self.degree_bits)?;
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
        self.openings.to_buffer(buffer)?;
        Ok(())
    }

    /// Deserializes a STARK proof.
    pub fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let degree_bits = buffer.read_target()?;
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
        let openings = StarkOpeningSetTarget::from_buffer(buffer)?;

        Ok(Self {
            trace_cap,
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
            degree_bits,
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

/// Merkle caps and openings that form the proof of a single STARK, along with its public inputs.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(bound = "")]
pub struct StarkProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    /// A STARK proof.
    pub proof: StarkProof<F, C, D>,
    /// Public inputs associated to this STARK proof.
    // TODO: Maybe make it generic over a `S: Stark` and replace with `[F; S::PUBLIC_INPUTS]`.
    pub public_inputs: Vec<F>,
}

/// Circuit version of [`StarkProofWithPublicInputs`].
#[derive(Debug, Clone)]
pub struct StarkProofWithPublicInputsTarget<const D: usize> {
    /// `Target` STARK proof.
    pub proof: StarkProofTarget<D>,
    /// `Target` public inputs for this STARK proof.
    pub public_inputs: Vec<Target>,
}

/// A [`StarkProof`] along with metadata about the initial Fiat-Shamir state, which is used when
/// creating a recursive wrapper proof around a STARK proof.
#[derive(Debug, Clone)]
pub struct StarkProofWithMetadata<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Initial Fiat-Shamir state.
    pub init_challenger_state: <C::Hasher as Hasher<F>>::Permutation,
    /// Proof for a single STARK.
    pub proof: StarkProof<F, C, D>,
}

/// Randomness used for a STARK proof.
#[derive(Debug)]
pub struct StarkProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    /// Optional randomness used in any permutation argument.
    pub lookup_challenge_set: Option<GrandProductChallengeSet<F>>,
    /// Random values used to combine STARK constraints.
    pub stark_alphas: Vec<F>,
    /// Point at which the STARK polynomials are opened.
    pub stark_zeta: F::Extension,
    /// Randomness used in FRI.
    pub fri_challenges: FriChallenges<F, D>,
}

/// Circuit version of [`StarkProofChallenges`].
#[derive(Debug)]
pub struct StarkProofChallengesTarget<const D: usize> {
    /// Optional `Target`'s randomness used in any permutation argument.
    pub lookup_challenge_set: Option<GrandProductChallengeSet<Target>>,
    /// `Target`s for the random values used to combine STARK constraints.
    pub stark_alphas: Vec<Target>,
    /// `ExtensionTarget` for the point at which the STARK polynomials are opened.
    pub stark_zeta: ExtensionTarget<D>,
    /// `Target`s for the randomness used in FRI.
    pub fri_challenges: FriChallengesTarget<D>,
}
/// Randomness for all STARK proofs contained in a MultiProof.
// TODO: remove?
#[derive(Debug)]
pub struct MultiProofChallenges<F: RichField + Extendable<D>, const D: usize, const N: usize> {
    /// Randomness used in each STARK proof.
    pub stark_challenges: [StarkProofChallenges<F, D>; N],
    /// Randomness used for cross-table lookups. It is shared by all STARKs.
    pub ctl_challenges: GrandProductChallengeSet<F>,
}

/// Purported values of each polynomial at the challenge point.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(bound = "")]
pub struct StarkOpeningSet<F: RichField + Extendable<D>, const D: usize> {
    /// Openings of trace polynomials at `zeta`.
    pub local_values: Vec<F::Extension>,
    /// Openings of trace polynomials at `g * zeta`.
    pub next_values: Vec<F::Extension>,
    /// Openings of lookups and cross-table lookups `Z` polynomials at `zeta`.
    pub auxiliary_polys: Option<Vec<F::Extension>>,
    /// Openings of lookups and cross-table lookups `Z` polynomials at `g * zeta`.
    pub auxiliary_polys_next: Option<Vec<F::Extension>>,
    /// Openings of cross-table lookups `Z` polynomials at `1`.
    pub ctl_zs_first: Option<Vec<F>>,
    /// Openings of quotient polynomials at `zeta`.
    pub quotient_polys: Option<Vec<F::Extension>>,
}

impl<F: RichField + Extendable<D>, const D: usize> StarkOpeningSet<F, D> {
    /// Returns a `StarkOpeningSet` given all the polynomial commitments, the number
    /// of permutation `Z`polynomials, the evaluation point and a generator `g`.
    ///
    /// Polynomials are evaluated at point `zeta` and, if necessary, at `g * zeta`.
    pub fn new<C: GenericConfig<D, F = F>>(
        zeta: F::Extension,
        g: F,
        trace_commitment: &PolynomialBatch<F, C, D>,
        auxiliary_polys_commitment: Option<&PolynomialBatch<F, C, D>>,
        quotient_commitment: Option<&PolynomialBatch<F, C, D>>,
        num_lookup_columns: usize,
        requires_ctl: bool,
        num_ctl_polys: &[usize],
    ) -> Self {
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
        // `g * zeta`.
        let zeta_next = zeta.scalar_mul(g);
        Self {
            local_values: eval_commitment(zeta, trace_commitment),
            next_values: eval_commitment(zeta_next, trace_commitment),
            auxiliary_polys: auxiliary_polys_commitment.map(|c| eval_commitment(zeta, c)),
            auxiliary_polys_next: auxiliary_polys_commitment.map(|c| eval_commitment(zeta_next, c)),
            ctl_zs_first: requires_ctl.then(|| {
                let total_num_helper_cols: usize = num_ctl_polys.iter().sum();
                auxiliary_first.unwrap()[num_lookup_columns + total_num_helper_cols..].to_vec()
            }),
            quotient_polys: quotient_commitment.map(|c| eval_commitment(zeta, c)),
        }
    }

    /// Constructs the openings required by FRI.
    /// All openings but `ctl_zs_first` are grouped together.
    pub(crate) fn to_fri_openings(&self) -> FriOpenings<F, D> {
        let zeta_batch = FriOpeningBatch {
            values: self
                .local_values
                .iter()
                .chain(self.auxiliary_polys.iter().flatten())
                .chain(self.quotient_polys.iter().flatten())
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

        let mut batches = vec![zeta_batch, zeta_next_batch];

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

            batches.push(ctl_first_batch);
        }

        FriOpenings { batches }
    }
}

/// Circuit version of [`StarkOpeningSet`].
/// `Target`s for the purported values of each polynomial at the challenge point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StarkOpeningSetTarget<const D: usize> {
    /// `ExtensionTarget`s for the openings of trace polynomials at `zeta`.
    pub local_values: Vec<ExtensionTarget<D>>,
    /// `ExtensionTarget`s for the opening of trace polynomials at `g * zeta`.
    pub next_values: Vec<ExtensionTarget<D>>,
    /// `ExtensionTarget`s for the opening of lookups and cross-table lookups `Z` polynomials at `zeta`.
    pub auxiliary_polys: Option<Vec<ExtensionTarget<D>>>,
    /// `ExtensionTarget`s for the opening of lookups and cross-table lookups `Z` polynomials at `g * zeta`.
    pub auxiliary_polys_next: Option<Vec<ExtensionTarget<D>>>,
    /// `ExtensionTarget`s for the opening of lookups and cross-table lookups `Z` polynomials at 1.
    pub ctl_zs_first: Option<Vec<Target>>,
    /// `ExtensionTarget`s for the opening of quotient polynomials at `zeta`.
    pub quotient_polys: Option<Vec<ExtensionTarget<D>>>,
}

impl<const D: usize> StarkOpeningSetTarget<D> {
    /// Serializes a STARK's opening set.
    pub(crate) fn to_buffer(&self, buffer: &mut Vec<u8>) -> IoResult<()> {
        buffer.write_target_ext_vec(&self.local_values)?;
        buffer.write_target_ext_vec(&self.next_values)?;
        if let Some(poly) = &self.auxiliary_polys {
            buffer.write_bool(true)?;
            buffer.write_target_ext_vec(poly)?;
        } else {
            buffer.write_bool(false)?;
        }
        if let Some(poly_next) = &self.auxiliary_polys_next {
            buffer.write_bool(true)?;
            buffer.write_target_ext_vec(poly_next)?;
        } else {
            buffer.write_bool(false)?;
        }
        if let Some(ctl_zs_first) = &self.ctl_zs_first {
            buffer.write_bool(true)?;
            buffer.write_target_vec(ctl_zs_first)?;
        } else {
            buffer.write_bool(false)?;
        }
        buffer.write_bool(self.quotient_polys.is_some())?;
        if let Some(quotient_polys) = &self.quotient_polys {
            buffer.write_target_ext_vec(quotient_polys)?;
        }
        Ok(())
    }

    /// Deserializes a STARK's opening set.
    pub(crate) fn from_buffer(buffer: &mut Buffer) -> IoResult<Self> {
        let local_values = buffer.read_target_ext_vec::<D>()?;
        let next_values = buffer.read_target_ext_vec::<D>()?;
        let auxiliary_polys = if buffer.read_bool()? {
            Some(buffer.read_target_ext_vec::<D>()?)
        } else {
            None
        };
        let auxiliary_polys_next = if buffer.read_bool()? {
            Some(buffer.read_target_ext_vec::<D>()?)
        } else {
            None
        };
        let ctl_zs_first = if buffer.read_bool()? {
            Some(buffer.read_target_vec()?)
        } else {
            None
        };
        let quotient_polys = if buffer.read_bool()? {
            Some(buffer.read_target_ext_vec::<D>()?)
        } else {
            None
        };

        Ok(Self {
            local_values,
            next_values,
            auxiliary_polys,
            auxiliary_polys_next,
            ctl_zs_first,
            quotient_polys,
        })
    }

    /// Circuit version of `to_fri_openings`for [`FriOpeningsTarget`].
    pub(crate) fn to_fri_openings(&self, zero: Target) -> FriOpeningsTarget<D> {
        let zeta_batch = FriOpeningBatchTarget {
            values: self
                .local_values
                .iter()
                .chain(self.auxiliary_polys.iter().flatten())
                .chain(self.quotient_polys.iter().flatten())
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

        let mut batches = vec![zeta_batch, zeta_next_batch];

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

            batches.push(ctl_first_batch);
        }
        FriOpeningsTarget { batches }
    }
}
