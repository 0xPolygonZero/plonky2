use crate::fri::reduction_strategies::FriReductionStrategy;

mod challenges;
pub mod oracle;
pub mod proof;
pub mod prover;
pub mod recursive_verifier;
pub mod reduction_strategies;
pub mod structure;
pub mod verifier;
pub mod witness_util;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FriConfig {
    /// `rate = 2^{-rate_bits}`.
    pub rate_bits: usize,

    /// Height of Merkle tree caps.
    pub cap_height: usize,

    pub proof_of_work_bits: u32,

    pub reduction_strategy: FriReductionStrategy,

    /// Number of query rounds to perform.
    pub num_query_rounds: usize,
}

impl FriConfig {
    pub fn rate(&self) -> f64 {
        1.0 / ((1 << self.rate_bits) as f64)
    }
}

/// FRI parameters, including generated parameters which are specific to an instance size, in
/// contrast to `FriConfig` which is user-specified and independent of instance size.
#[derive(Debug)]
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
    pub(crate) fn total_arities(&self) -> usize {
        self.reduction_arity_bits.iter().sum()
    }

    pub(crate) fn max_arity_bits(&self) -> Option<usize> {
        self.reduction_arity_bits.iter().copied().max()
    }

    pub fn lde_bits(&self) -> usize {
        self.degree_bits + self.config.rate_bits
    }

    pub fn lde_size(&self) -> usize {
        1 << self.lde_bits()
    }

    pub fn final_poly_bits(&self) -> usize {
        self.degree_bits - self.total_arities()
    }

    pub fn final_poly_len(&self) -> usize {
        1 << self.final_poly_bits()
    }
}
