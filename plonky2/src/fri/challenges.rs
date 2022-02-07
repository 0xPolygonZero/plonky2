use plonky2_field::extension_field::Extendable;
use plonky2_field::polynomial::PolynomialCoeffs;

use crate::fri::proof::{FriChallenges, FriChallengesTarget};
use crate::fri::structure::{FriOpenings, FriOpeningsTarget};
use crate::fri::FriConfig;
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::hash::hash_types::{MerkleCapTarget, RichField};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::challenger::{Challenger, RecursiveChallenger};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
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
                self.observe_cap(cap);
                self.get_extension_challenge::<D>()
            })
            .collect();

        self.observe_extension_elements(&final_poly.coeffs);

        let fri_pow_response = C::InnerHasher::hash_no_pad(
            &self
                .get_hash()
                .elements
                .iter()
                .copied()
                .chain(Some(pow_witness))
                .collect::<Vec<_>>(),
        )
        .elements[0];

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

    pub fn fri_challenges<C: GenericConfig<D, F = F>>(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        commit_phase_merkle_caps: &[MerkleCapTarget],
        final_poly: &PolynomialCoeffsExtTarget<D>,
        pow_witness: Target,
        inner_common_data: &CommonCircuitData<F, C, D>,
    ) -> FriChallengesTarget<D> {
        let num_fri_queries = inner_common_data.config.fri_config.num_query_rounds;
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

        let pow_inputs = self
            .get_hash(builder)
            .elements
            .iter()
            .copied()
            .chain(Some(pow_witness))
            .collect();
        let fri_pow_response = builder
            .hash_n_to_hash_no_pad::<C::InnerHasher>(pow_inputs)
            .elements[0];

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
