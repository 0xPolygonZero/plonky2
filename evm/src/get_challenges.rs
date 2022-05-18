use plonky2::field::extension_field::Extendable;
use plonky2::fri::proof::FriProof;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};

use crate::all_stark::AllStark;
use crate::config::StarkConfig;
use crate::permutation::{
    get_grand_product_challenge_set, get_n_grand_product_challenge_sets,
    get_n_permutation_challenge_sets_target,
};
use crate::proof::*;
use crate::stark::Stark;

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> AllProof<F, C, D> {
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
    ) -> AllProofChallenges<F, D> {
        let mut challenger = Challenger::<F, C::Hasher>::new();

        for proof in self.proofs() {
            challenger.observe_cap(&proof.proof.trace_cap);
        }

        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, config.num_challenges);

        AllProofChallenges {
            cpu_challenges: self.cpu_proof.get_challenges(
                &mut challenger,
                &all_stark.cpu_stark,
                config,
            ),
            keccak_challenges: self.keccak_proof.get_challenges(
                &mut challenger,
                &all_stark.keccak_stark,
                config,
            ),
            ctl_challenges,
        }
    }
}

impl<F, C, const D: usize> StarkProofWithPublicInputs<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges<S: Stark<F, D>>(
        &self,
        challenger: &mut Challenger<F, C::Hasher>,
        stark: &S,
        config: &StarkConfig,
    ) -> StarkProofChallenges<F, D> {
        let degree_bits = self.proof.recover_degree_bits(config);

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
        } = &self.proof;

        let num_challenges = config.num_challenges;

        let permutation_challenge_sets = stark.uses_permutation_args().then(|| {
            get_n_grand_product_challenge_sets(
                challenger,
                num_challenges,
                stark.permutation_batch_size(),
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

impl<const D: usize> StarkProofWithPublicInputsTarget<D> {
    pub(crate) fn get_challenges<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        S: Stark<F, D>,
    >(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        stark: &S,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let proof = &self.proof;
        let opening_proof = &proof.opening_proof;
        let num_challenges = config.num_challenges;
        let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::new(builder);
        challenger.observe_cap(&proof.trace_cap);
        let permutation_challenge_sets =
            proof.permutation_zs_cap.as_ref().map(|permutation_zs_cap| {
                let tmp = get_n_permutation_challenge_sets_target(
                    builder,
                    &mut challenger,
                    num_challenges,
                    stark.permutation_batch_size(),
                );
                challenger.observe_cap(permutation_zs_cap);
                tmp
            });
        let stark_alphas = challenger.get_n_challenges(builder, num_challenges);
        challenger.observe_cap(&proof.quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge(builder);
        challenger.observe_openings(&proof.openings.to_fri_openings());
        StarkProofChallengesTarget {
            permutation_challenge_sets,
            stark_alphas,
            stark_zeta,
            fri_challenges: challenger.fri_challenges::<C>(
                builder,
                &opening_proof.commit_phase_merkle_caps,
                &opening_proof.final_poly,
                opening_proof.pow_witness,
                &config.fri_config,
            ),
        }
    }
}
