//! Fast Reed-Solomon IOP (FRI) protocol.
//!
//! It provides both a native implementation and an in-circuit version
//! of the FRI verifier for recursive proof composition.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use plonky2_field::extension::Extendable;
use serde::Serialize;

use crate::fri::reduction_strategies::FriReductionStrategy;
use crate::hash::hash_types::RichField;
use crate::iop::challenger::{Challenger, RecursiveChallenger};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{AlgebraicHasher, Hasher};

mod challenges;
pub mod oracle;
pub mod proof;
pub mod prover;
pub mod recursive_verifier;
pub mod reduction_strategies;
pub mod structure;
pub(crate) mod validate_shape;
pub mod verifier;
pub mod witness_util;

/// A configuration for the FRI protocol.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct FriConfig {
    /// `rate = 2^{-rate_bits}`.
    pub rate_bits: usize,

    /// Height of Merkle tree caps.
    pub cap_height: usize,

    /// Number of bits used for grinding.
    pub proof_of_work_bits: u32,

    /// The reduction strategy to be applied at each layer during the commit phase.
    pub reduction_strategy: FriReductionStrategy,

    /// Number of query rounds to perform.
    pub num_query_rounds: usize,
}

impl FriConfig {
    pub fn rate(&self) -> f64 {
        1.0 / ((1 << self.rate_bits) as f64)
    }

    pub fn fri_params(&self, degree_bits: usize, hiding: bool) -> FriParams {
        let reduction_arity_bits = self.reduction_strategy.reduction_arity_bits(
            degree_bits,
            self.rate_bits,
            self.cap_height,
            self.num_query_rounds,
        );
        FriParams {
            config: self.clone(),
            hiding,
            degree_bits,
            reduction_arity_bits,
        }
    }

    pub const fn num_cap_elements(&self) -> usize {
        1 << self.cap_height
    }

    /// Observe the FRI configuration parameters.
    pub fn observe<F: RichField, H: Hasher<F>>(&self, challenger: &mut Challenger<F, H>) {
        challenger.observe_element(F::from_canonical_usize(self.rate_bits));
        challenger.observe_element(F::from_canonical_usize(self.cap_height));
        challenger.observe_element(F::from_canonical_u32(self.proof_of_work_bits));
        challenger.observe_elements(&self.reduction_strategy.serialize());
        challenger.observe_element(F::from_canonical_usize(self.num_query_rounds));
    }

    /// Observe the FRI configuration parameters for the recursive verifier.
    pub fn observe_target<F, H, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, H, D>,
    ) where
        F: RichField + Extendable<D>,
        H: AlgebraicHasher<F>,
    {
        challenger.observe_element(builder.constant(F::from_canonical_usize(self.rate_bits)));
        challenger.observe_element(builder.constant(F::from_canonical_usize(self.cap_height)));
        challenger
            .observe_element(builder.constant(F::from_canonical_u32(self.proof_of_work_bits)));
        challenger.observe_elements(&builder.constants(&self.reduction_strategy.serialize()));
        challenger
            .observe_element(builder.constant(F::from_canonical_usize(self.num_query_rounds)));
    }
}

/// FRI parameters, including generated parameters which are specific to an instance size, in
/// contrast to `FriConfig` which is user-specified and independent of instance size.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct FriParams {
    /// User-specified FRI configuration.
    pub config: FriConfig,

    /// Whether to use a hiding variant of Merkle trees (where random salts are added to leaves).
    pub hiding: bool,

    /// The degree of the purported codeword, measured in bits.
    pub degree_bits: usize,

    /// The arity of each FRI reduction step, expressed as the log2 of the actual arity.
    /// For example, `[3, 2, 1]` would describe a FRI reduction tree with 8-to-1 reduction, then
    /// a 4-to-1 reduction, then a 2-to-1 reduction. After these reductions, the reduced polynomial
    /// is sent directly.
    pub reduction_arity_bits: Vec<usize>,
}

impl FriParams {
    pub fn total_arities(&self) -> usize {
        self.reduction_arity_bits.iter().sum()
    }

    pub(crate) fn max_arity_bits(&self) -> Option<usize> {
        self.reduction_arity_bits.iter().copied().max()
    }

    pub const fn lde_bits(&self) -> usize {
        self.degree_bits + self.config.rate_bits
    }

    pub const fn lde_size(&self) -> usize {
        1 << self.lde_bits()
    }

    pub fn final_poly_bits(&self) -> usize {
        self.degree_bits - self.total_arities()
    }

    pub fn final_poly_len(&self) -> usize {
        1 << self.final_poly_bits()
    }

    pub fn observe<F: RichField, H: Hasher<F>>(&self, challenger: &mut Challenger<F, H>) {
        self.config.observe(challenger);

        challenger.observe_element(F::from_bool(self.hiding));
        challenger.observe_element(F::from_canonical_usize(self.degree_bits));
        challenger.observe_elements(
            &self
                .reduction_arity_bits
                .iter()
                .map(|&e| F::from_canonical_usize(e))
                .collect::<Vec<_>>(),
        );
    }

    pub fn observe_target<F, H, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, H, D>,
    ) where
        F: RichField + Extendable<D>,
        H: AlgebraicHasher<F>,
    {
        self.config.observe_target(builder, challenger);

        challenger.observe_element(builder.constant(F::from_bool(self.hiding)));
        challenger.observe_element(builder.constant(F::from_canonical_usize(self.degree_bits)));
        challenger.observe_elements(
            &builder.constants(
                &self
                    .reduction_arity_bits
                    .iter()
                    .map(|&e| F::from_canonical_usize(e))
                    .collect::<Vec<_>>(),
            ),
        );
    }
}
