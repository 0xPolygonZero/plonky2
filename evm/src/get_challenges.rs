use plonky2::field::extension::Extendable;
use plonky2::fri::proof::{FriProof, FriProofTarget};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};

use crate::all_stark::AllStark;
use crate::config::StarkConfig;
use crate::permutation::{
    get_grand_product_challenge_set, get_grand_product_challenge_set_target,
    get_n_grand_product_challenge_sets, get_n_grand_product_challenge_sets_target,
};
use crate::proof::*;

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> AllProof<F, C, D> {
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
    ) -> AllProofChallenges<F, D> {
        let mut challenger = Challenger::<F, C::Hasher>::new();

        for proof in &self.stark_proofs {
            challenger.observe_cap(&proof.trace_cap);
        }

        // TODO: Observe public values.

        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, config.num_challenges);

        let num_permutation_zs = all_stark.nums_permutation_zs(config);
        let num_permutation_batch_sizes = all_stark.permutation_batch_sizes();

        AllProofChallenges {
            stark_challenges: std::array::from_fn(|i| {
                self.stark_proofs[i].get_challenges(
                    &mut challenger,
                    num_permutation_zs[i] > 0,
                    num_permutation_batch_sizes[i],
                    config,
                )
            }),
            ctl_challenges,
        }
    }
}

impl<const D: usize> AllProofTarget<D> {
    pub(crate) fn get_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
    ) -> AllProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::new(builder);

        for proof in &self.stark_proofs {
            challenger.observe_cap(&proof.trace_cap);
        }

        let ctl_challenges =
            get_grand_product_challenge_set_target(builder, &mut challenger, config.num_challenges);

        let num_permutation_zs = all_stark.nums_permutation_zs(config);
        let num_permutation_batch_sizes = all_stark.permutation_batch_sizes();

        AllProofChallengesTarget {
            stark_challenges: std::array::from_fn(|i| {
                self.stark_proofs[i].get_challenges::<F, C>(
                    builder,
                    &mut challenger,
                    num_permutation_zs[i] > 0,
                    num_permutation_batch_sizes[i],
                    config,
                )
            }),
            ctl_challenges,
        }
    }
}

impl<F, C, const D: usize> StarkProof<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges(
        &self,
        challenger: &mut Challenger<F, C::Hasher>,
        stark_use_permutation: bool,
        stark_permutation_batch_size: usize,
        config: &StarkConfig,
    ) -> StarkProofChallenges<F, D> {
        let degree_bits = self.recover_degree_bits(config);

        let StarkProof {
            permutation_ctl_zs_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProof {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
            ..
        } = &self;

        let num_challenges = config.num_challenges;

        let permutation_challenge_sets = stark_use_permutation.then(|| {
            get_n_grand_product_challenge_sets(
                challenger,
                num_challenges,
                stark_permutation_batch_size,
            )
        });

        challenger.observe_cap(permutation_ctl_zs_cap);

        let stark_alphas = challenger.get_n_challenges(num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge::<D>();

        challenger.observe_openings(&openings.to_fri_openings());

        StarkProofChallenges {
            permutation_challenge_sets,
            stark_alphas,
            stark_zeta,
            fri_challenges: challenger.fri_challenges::<C, D>(
                commit_phase_merkle_caps,
                final_poly,
                *pow_witness,
                degree_bits,
                &config.fri_config,
            ),
        }
    }
}

impl<const D: usize> StarkProofTarget<D> {
    pub(crate) fn get_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
        stark_use_permutation: bool,
        stark_permutation_batch_size: usize,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let StarkProofTarget {
            permutation_ctl_zs_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProofTarget {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
            ..
        } = &self;

        let num_challenges = config.num_challenges;

        let permutation_challenge_sets = stark_use_permutation.then(|| {
            get_n_grand_product_challenge_sets_target(
                builder,
                challenger,
                num_challenges,
                stark_permutation_batch_size,
            )
        });

        challenger.observe_cap(permutation_ctl_zs_cap);

        let stark_alphas = challenger.get_n_challenges(builder, num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge(builder);

        challenger.observe_openings(&openings.to_fri_openings(builder.zero()));

        StarkProofChallengesTarget {
            permutation_challenge_sets,
            stark_alphas,
            stark_zeta,
            fri_challenges: challenger.fri_challenges::<C>(
                builder,
                commit_phase_merkle_caps,
                final_poly,
                *pow_witness,
                &config.fri_config,
            ),
        }
    }
}
