//! A [`StarkConfig`] defines all the parameters to be used when proving a
//! [`Stark`][crate::stark::Stark].
//!
//! The default configuration is aimed for speed, yielding fast but large
//! proofs, with a targeted security level of 100 bits.

#[cfg(not(feature = "std"))]
use alloc::format;

use anyhow::{anyhow, Result};
use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::fri::reduction_strategies::FriReductionStrategy;
use plonky2::fri::{FriConfig, FriParams};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, Hasher};

/// A configuration containing the different parameters used by the STARK prover.
#[derive(Clone, Debug)]
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
    /// Returns a custom STARK configuration.
    pub const fn new(security_bits: usize, num_challenges: usize, fri_config: FriConfig) -> Self {
        Self {
            security_bits,
            num_challenges,
            fri_config,
        }
    }

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

    /// Outputs the [`FriParams`] used during the FRI sub-protocol by this [`StarkConfig`].
    pub fn fri_params(&self, degree_bits: usize) -> FriParams {
        self.fri_config.fri_params(degree_bits, false)
    }

    /// Checks that this STARK configuration is consistent, i.e. that the different
    /// parameters meet the targeted security level.
    pub fn check_config<F: RichField + Extendable<D>, const D: usize>(&self) -> Result<()> {
        let StarkConfig {
            security_bits,
            fri_config:
                FriConfig {
                    rate_bits,
                    proof_of_work_bits,
                    num_query_rounds,
                    ..
                },
            ..
        } = &self;

        // Conjectured FRI security; see the ethSTARK paper.
        let fri_field_bits = F::Extension::order().bits() as usize;
        let fri_query_security_bits = num_query_rounds * rate_bits + *proof_of_work_bits as usize;
        let fri_security_bits = fri_field_bits.min(fri_query_security_bits);

        if fri_security_bits < *security_bits {
            Err(anyhow!(format!(
                "FRI params fall short of target security {}, reaching only {}",
                security_bits, fri_security_bits
            )))
        } else {
            Ok(())
        }
    }

    /// Observes this [`StarkConfig`] for the given [`Challenger`].
    pub fn observe<F: RichField, H: Hasher<F>>(&self, challenger: &mut Challenger<F, H>) {
        challenger.observe_element(F::from_canonical_usize(self.security_bits));
        challenger.observe_element(F::from_canonical_usize(self.num_challenges));

        self.fri_config.observe(challenger);
    }

    /// Observes this [`StarkConfig`] for the given [`RecursiveChallenger`].
    pub(crate) fn observe_target<F, H, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, H, D>,
    ) where
        F: RichField + Extendable<D>,
        H: AlgebraicHasher<F>,
    {
        challenger.observe_element(builder.constant(F::from_canonical_usize(self.security_bits)));
        challenger.observe_element(builder.constant(F::from_canonical_usize(self.num_challenges)));

        self.fri_config.observe_target(builder, challenger);
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;

    use super::*;

    #[test]
    fn test_valid_config() {
        type F = GoldilocksField;
        const D: usize = 2;

        let config = StarkConfig::standard_fast_config();
        assert!(config.check_config::<F, D>().is_ok());

        let high_rate_config = StarkConfig::new(
            100,
            2,
            FriConfig {
                rate_bits: 3,
                cap_height: 4,
                proof_of_work_bits: 16,
                reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
                num_query_rounds: 28,
            },
        );
        assert!(high_rate_config.check_config::<F, D>().is_ok());
    }

    #[test]
    fn test_invalid_config() {
        type F = GoldilocksField;
        const D: usize = 2;

        let too_few_queries_config = StarkConfig::new(
            100,
            2,
            FriConfig {
                rate_bits: 1,
                cap_height: 4,
                proof_of_work_bits: 16,
                reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
                num_query_rounds: 50,
            },
        );
        // The conjectured security yields `rate_bits` * `num_query_rounds` + `proof_of_work_bits` = 66
        // bits of security for FRI, which falls short of the 100 bits of security target.
        assert!(too_few_queries_config.check_config::<F, D>().is_err());
    }
}
