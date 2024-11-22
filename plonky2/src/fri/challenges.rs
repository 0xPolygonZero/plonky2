#[cfg(not(feature = "std"))]
use alloc::vec;

use crate::field::extension::Extendable;
use crate::field::polynomial::PolynomialCoeffs;
use crate::field::types::Field;
use crate::fri::proof::{FriChallenges, FriChallengesTarget};
use crate::fri::structure::{FriOpenings, FriOpeningsTarget};
use crate::fri::FriConfig;
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::hash::hash_types::{MerkleCapTarget, RichField, NUM_HASH_OUT_ELTS};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::challenger::{Challenger, RecursiveChallenger};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};

impl<F: RichField, H: Hasher<F>> Challenger<F, H> {
    pub fn observe_openings<const D: usize>(&mut self, openings: &FriOpenings<F, D>)
    where
        F: RichField + Extendable<D>,
    {
        for v in &openings.batches {
            self.observe_extension_elements(&v.values);
        }
    }

    pub fn fri_challenges<C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        commit_phase_merkle_caps: &[MerkleCap<F, C::Hasher>],
        final_poly: &PolynomialCoeffs<F::Extension>,
        pow_witness: F,
        degree_bits: usize,
        config: &FriConfig,
        final_poly_coeff_len: Option<usize>,
        max_num_query_steps: Option<usize>,
    ) -> FriChallenges<F, D>
    where
        F: RichField + Extendable<D>,
    {
        let num_fri_queries = config.num_query_rounds;
        let lde_size = 1 << (degree_bits + config.rate_bits);
        // Scaling factor to combine polynomials.
        let fri_alpha = self.get_extension_challenge::<D>();

        // Recover the random betas used in the FRI reductions.
        let fri_betas = commit_phase_merkle_caps
            .iter()
            .map(|cap| {
                self.observe_cap::<C::Hasher>(cap);
                self.get_extension_challenge::<D>()
            })
            .collect();

        // When this proof was generated in a circuit with a different number of query steps,
        // the challenger needs to observe the additional hash caps.
        if let Some(step_count) = max_num_query_steps {
            let cap_len = (1 << config.cap_height) * NUM_HASH_OUT_ELTS;
            let zero_cap = vec![F::ZERO; cap_len];
            for _ in commit_phase_merkle_caps.len()..step_count {
                self.observe_elements(&zero_cap);
                self.get_extension_challenge::<D>();
            }
        }

        self.observe_extension_elements(&final_poly.coeffs);
        // When this proof was generated in a circuit with a different final polynomial length,
        // the challenger needs to observe the full length of the final polynomial.
        if let Some(len) = final_poly_coeff_len {
            let current_len = final_poly.coeffs.len();
            for _ in current_len..len {
                self.observe_extension_element(&F::Extension::ZERO);
            }
        }

        self.observe_element(pow_witness);
        let fri_pow_response = self.get_challenge();

        let fri_query_indices = (0..num_fri_queries)
            .map(|_| self.get_challenge().to_canonical_u64() as usize % lde_size)
            .collect();

        FriChallenges {
            fri_alpha,
            fri_betas,
            fri_pow_response,
            fri_query_indices,
        }
    }
}

impl<F: RichField + Extendable<D>, H: AlgebraicHasher<F>, const D: usize>
    RecursiveChallenger<F, H, D>
{
    pub fn observe_openings(&mut self, openings: &FriOpeningsTarget<D>) {
        for v in &openings.batches {
            self.observe_extension_elements(&v.values);
        }
    }

    pub fn fri_challenges(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        commit_phase_merkle_caps: &[MerkleCapTarget],
        final_poly: &PolynomialCoeffsExtTarget<D>,
        pow_witness: Target,
        inner_fri_config: &FriConfig,
    ) -> FriChallengesTarget<D> {
        let num_fri_queries = inner_fri_config.num_query_rounds;
        // Scaling factor to combine polynomials.
        let fri_alpha = self.get_extension_challenge(builder);

        // Recover the random betas used in the FRI reductions.
        let fri_betas = commit_phase_merkle_caps
            .iter()
            .map(|cap| {
                self.observe_cap(cap);
                self.get_extension_challenge(builder)
            })
            .collect();

        self.observe_extension_elements(&final_poly.0);

        self.observe_element(pow_witness);
        let fri_pow_response = self.get_challenge(builder);

        let fri_query_indices = (0..num_fri_queries)
            .map(|_| self.get_challenge(builder))
            .collect();

        FriChallengesTarget {
            fri_alpha,
            fri_betas,
            fri_pow_response,
            fri_query_indices,
        }
    }
}
