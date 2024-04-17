use plonky2::fri::reduction_strategies::FriReductionStrategy;
use plonky2::fri::{FriConfig, FriParams};

/// A configuration containing the different parameters to be used by the STARK prover.
pub struct StarkConfig {
    /// The targeted security level for the proofs generated with this configuration.
    pub security_bits: usize,

    /// The number of challenge points to generate, for IOPs that have soundness errors of (roughly)
    /// `degree / |F|`.
    pub num_challenges: usize,

    /// The configuration of the FRI sub-protocol.
    pub fri_config: FriConfig,
}

impl Default for StarkConfig {
    fn default() -> Self {
        Self::standard_fast_config()
    }
}

impl StarkConfig {
    /// A typical configuration with a rate of 2, resulting in fast but large proofs.
    /// Targets ~100 bit conjectured security.
    pub const fn standard_fast_config() -> Self {
        Self {
            security_bits: 100,
            num_challenges: 2,
            fri_config: FriConfig {
                rate_bits: 1,
                cap_height: 4,
                proof_of_work_bits: 16,
                reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
                num_query_rounds: 84,
            },
        }
    }

    pub(crate) fn fri_params(&self, degree_bits: usize) -> FriParams {
        self.fri_config.fri_params(degree_bits, false)
    }
}
