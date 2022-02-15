use std::collections::HashSet;

use plonky2_field::extension_field::Extendable;
use plonky2_field::polynomial::PolynomialCoeffs;

use crate::fri::proof::{CompressedFriProof, FriChallenges, FriProof, FriProofTarget};
use crate::fri::verifier::{compute_evaluation, fri_combine_initial, PrecomputedReducedOpenings};
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::hash::hash_types::{HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::challenger::{Challenger, RecursiveChallenger};
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use crate::plonk::proof::{
    CompressedProof, CompressedProofWithPublicInputs, FriInferredElements, OpeningSet,
    OpeningSetTarget, Proof, ProofChallenges, ProofChallengesTarget, ProofTarget,
    ProofWithPublicInputs, ProofWithPublicInputsTarget,
};
use crate::util::reverse_bits;

fn get_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    public_inputs_hash: <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash,
    wires_cap: &MerkleCap<F, C::Hasher>,
    plonk_zs_partial_products_cap: &MerkleCap<F, C::Hasher>,
    quotient_polys_cap: &MerkleCap<F, C::Hasher>,
    openings: &OpeningSet<F, D>,
    commit_phase_merkle_caps: &[MerkleCap<F, C::Hasher>],
    final_poly: &PolynomialCoeffs<F::Extension>,
    pow_witness: F,
    common_data: &CommonCircuitData<F, C, D>,
) -> anyhow::Result<ProofChallenges<F, D>> {
    let config = &common_data.config;
    let num_challenges = config.num_challenges;

    let mut challenger = Challenger::<F, C::Hasher>::new();

    // Observe the instance.
    challenger.observe_hash::<C::Hasher>(common_data.circuit_digest);
    challenger.observe_hash::<C::InnerHasher>(public_inputs_hash);

    challenger.observe_cap(wires_cap);
    let plonk_betas = challenger.get_n_challenges(num_challenges);
    let plonk_gammas = challenger.get_n_challenges(num_challenges);

    challenger.observe_cap(plonk_zs_partial_products_cap);
    let plonk_alphas = challenger.get_n_challenges(num_challenges);

    challenger.observe_cap(quotient_polys_cap);
    let plonk_zeta = challenger.get_extension_challenge::<D>();

    challenger.observe_openings(&openings.to_fri_openings());

    Ok(ProofChallenges {
        plonk_betas,
        plonk_gammas,
        plonk_alphas,
        plonk_zeta,
        fri_challenges: challenger.fri_challenges::<C, D>(
            commit_phase_merkle_caps,
            final_poly,
            pow_witness,
            common_data.degree_bits,
            &config.fri_config,
        ),
    })
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    ProofWithPublicInputs<F, C, D>
{
    pub(crate) fn fri_query_indices(
        &self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> anyhow::Result<Vec<usize>> {
        Ok(self
            .get_challenges(self.get_public_inputs_hash(), common_data)?
            .fri_challenges
            .fri_query_indices)
    }

    /// Computes all Fiat-Shamir challenges used in the Plonk proof.
    pub(crate) fn get_challenges(
        &self,
        public_inputs_hash: <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> anyhow::Result<ProofChallenges<F, D>> {
        let Proof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProof {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
        } = &self.proof;

        get_challenges(
            public_inputs_hash,
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            common_data,
        )
    }
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    CompressedProofWithPublicInputs<F, C, D>
{
    /// Computes all Fiat-Shamir challenges used in the Plonk proof.
    pub(crate) fn get_challenges(
        &self,
        public_inputs_hash: <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> anyhow::Result<ProofChallenges<F, D>> {
        let CompressedProof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                CompressedFriProof {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
        } = &self.proof;

        get_challenges(
            public_inputs_hash,
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            common_data,
        )
    }

    /// Computes all coset elements that can be inferred in the FRI reduction steps.
    pub(crate) fn get_inferred_elements(
        &self,
        challenges: &ProofChallenges<F, D>,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> FriInferredElements<F, D> {
        let ProofChallenges {
            plonk_zeta,
            fri_challenges:
                FriChallenges {
                    fri_alpha,
                    fri_betas,
                    fri_query_indices,
                    ..
                },
            ..
        } = challenges;
        let mut fri_inferred_elements = Vec::new();
        // Holds the indices that have already been seen at each reduction depth.
        let mut seen_indices_by_depth =
            vec![HashSet::new(); common_data.fri_params.reduction_arity_bits.len()];
        let precomputed_reduced_evals = PrecomputedReducedOpenings::from_os_and_alpha(
            &self.proof.openings.to_fri_openings(),
            *fri_alpha,
        );
        let log_n = common_data.degree_bits + common_data.config.fri_config.rate_bits;
        // Simulate the proof verification and collect the inferred elements.
        // The content of the loop is basically the same as the `fri_verifier_query_round` function.
        for &(mut x_index) in fri_query_indices {
            let mut subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
                * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(x_index, log_n) as u64);
            let mut old_eval = fri_combine_initial::<F, C, D>(
                &common_data.get_fri_instance(*plonk_zeta),
                &self
                    .proof
                    .opening_proof
                    .query_round_proofs
                    .initial_trees_proofs[&x_index],
                *fri_alpha,
                subgroup_x,
                &precomputed_reduced_evals,
                &common_data.fri_params,
            );
            for (i, &arity_bits) in common_data
                .fri_params
                .reduction_arity_bits
                .iter()
                .enumerate()
            {
                let coset_index = x_index >> arity_bits;
                if !seen_indices_by_depth[i].insert(coset_index) {
                    // If this index has already been seen, we can skip the rest of the reductions.
                    break;
                }
                fri_inferred_elements.push(old_eval);
                let arity = 1 << arity_bits;
                let mut evals = self.proof.opening_proof.query_round_proofs.steps[i][&coset_index]
                    .evals
                    .clone();
                let x_index_within_coset = x_index & (arity - 1);
                evals.insert(x_index_within_coset, old_eval);
                old_eval = compute_evaluation(
                    subgroup_x,
                    x_index_within_coset,
                    arity_bits,
                    &evals,
                    fri_betas[i],
                );
                subgroup_x = subgroup_x.exp_power_of_2(arity_bits);
                x_index = coset_index;
            }
        }
        FriInferredElements(fri_inferred_elements)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    fn get_challenges<C: GenericConfig<D, F = F>>(
        &mut self,
        public_inputs_hash: HashOutTarget,
        wires_cap: &MerkleCapTarget,
        plonk_zs_partial_products_cap: &MerkleCapTarget,
        quotient_polys_cap: &MerkleCapTarget,
        openings: &OpeningSetTarget<D>,
        commit_phase_merkle_caps: &[MerkleCapTarget],
        final_poly: &PolynomialCoeffsExtTarget<D>,
        pow_witness: Target,
        inner_common_data: &CommonCircuitData<F, C, D>,
    ) -> ProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let config = &inner_common_data.config;
        let num_challenges = config.num_challenges;

        let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::new(self);

        // Observe the instance.
        let digest =
            HashOutTarget::from_vec(self.constants(&inner_common_data.circuit_digest.elements));
        challenger.observe_hash(&digest);
        challenger.observe_hash(&public_inputs_hash);

        challenger.observe_cap(wires_cap);
        let plonk_betas = challenger.get_n_challenges(self, num_challenges);
        let plonk_gammas = challenger.get_n_challenges(self, num_challenges);

        challenger.observe_cap(plonk_zs_partial_products_cap);
        let plonk_alphas = challenger.get_n_challenges(self, num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let plonk_zeta = challenger.get_extension_challenge(self);

        challenger.observe_openings(&openings.to_fri_openings());

        ProofChallengesTarget {
            plonk_betas,
            plonk_gammas,
            plonk_alphas,
            plonk_zeta,
            fri_challenges: challenger.fri_challenges::<C>(
                self,
                commit_phase_merkle_caps,
                final_poly,
                pow_witness,
                inner_common_data,
            ),
        }
    }
}

impl<const D: usize> ProofWithPublicInputsTarget<D> {
    pub(crate) fn get_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        public_inputs_hash: HashOutTarget,
        inner_common_data: &CommonCircuitData<F, C, D>,
    ) -> ProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let ProofTarget {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProofTarget {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
        } = &self.proof;

        builder.get_challenges(
            public_inputs_hash,
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            inner_common_data,
        )
    }
}
