use anyhow::Result;
use plonky2::field::extension_field::Extendable;
use plonky2::field::polynomial::PolynomialCoeffs;
use plonky2::fri::proof::FriProof;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;

use crate::config::StarkConfig;
use crate::proof::{StarkOpeningSet, StarkProof, StarkProofChallenges, StarkProofWithPublicInputs};

fn get_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    trace_cap: &MerkleCap<F, C::Hasher>,
    quotient_polys_cap: &MerkleCap<F, C::Hasher>,
    openings: &StarkOpeningSet<F, D>,
    commit_phase_merkle_caps: &[MerkleCap<F, C::Hasher>],
    final_poly: &PolynomialCoeffs<F::Extension>,
    pow_witness: F,
    config: &StarkConfig,
    degree_bits: usize,
) -> Result<StarkProofChallenges<F, D>> {
    let num_challenges = config.num_challenges;
    let num_fri_queries = config.fri_config.num_query_rounds;
    let lde_size = 1 << (degree_bits + config.fri_config.rate_bits);

    let mut challenger = Challenger::<F, C::Hasher>::new();

    challenger.observe_cap(trace_cap);
    let stark_alphas = challenger.get_n_challenges(num_challenges);

    challenger.observe_cap(quotient_polys_cap);
    let stark_zeta = challenger.get_extension_challenge::<D>();

    openings.observe(&mut challenger);

    Ok(StarkProofChallenges {
        stark_alphas,
        stark_zeta,
        fri_challenges: challenger.fri_challenges::<C, D>(
            commit_phase_merkle_caps,
            final_poly,
            pow_witness,
            degree_bits,
            &config.fri_config,
        ),
    })
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    StarkProofWithPublicInputs<F, C, D>
{
    pub(crate) fn fri_query_indices(
        &self,
        config: &StarkConfig,
        degree_bits: usize,
    ) -> anyhow::Result<Vec<usize>> {
        Ok(self
            .get_challenges(config, degree_bits)?
            .fri_challenges
            .fri_query_indices)
    }

    /// Computes all Fiat-Shamir challenges used in the Plonk proof.
    pub(crate) fn get_challenges(
        &self,
        config: &StarkConfig,
        degree_bits: usize,
    ) -> Result<StarkProofChallenges<F, D>> {
        let StarkProof {
            trace_cap,
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

        get_challenges::<F, C, D>(
            trace_cap,
            quotient_polys_cap,
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            config,
            degree_bits,
        )
    }
}

// impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
//     CompressedProofWithPublicInputs<F, C, D>
// {
//     /// Computes all Fiat-Shamir challenges used in the Plonk proof.
//     pub(crate) fn get_challenges(
//         &self,
//         common_data: &CommonCircuitData<F, C, D>,
//     ) -> anyhow::Result<ProofChallenges<F, D>> {
//         let CompressedProof {
//             wires_cap,
//             plonk_zs_partial_products_cap,
//             quotient_polys_cap,
//             openings,
//             opening_proof:
//                 CompressedFriProof {
//                     commit_phase_merkle_caps,
//                     final_poly,
//                     pow_witness,
//                     ..
//                 },
//         } = &self.proof;
//
//         get_challenges(
//             self.get_public_inputs_hash(),
//             wires_cap,
//             plonk_zs_partial_products_cap,
//             quotient_polys_cap,
//             openings,
//             commit_phase_merkle_caps,
//             final_poly,
//             *pow_witness,
//             common_data,
//         )
//     }
//
//     /// Computes all coset elements that can be inferred in the FRI reduction steps.
//     pub(crate) fn get_inferred_elements(
//         &self,
//         challenges: &ProofChallenges<F, D>,
//         common_data: &CommonCircuitData<F, C, D>,
//     ) -> FriInferredElements<F, D> {
//         let ProofChallenges {
//             plonk_zeta,
//             fri_alpha,
//             fri_betas,
//             fri_query_indices,
//             ..
//         } = challenges;
//         let mut fri_inferred_elements = Vec::new();
//         // Holds the indices that have already been seen at each reduction depth.
//         let mut seen_indices_by_depth =
//             vec![HashSet::new(); common_data.fri_params.reduction_arity_bits.len()];
//         let precomputed_reduced_evals = PrecomputedReducedOpenings::from_os_and_alpha(
//             &self.proof.openings.to_fri_openings(),
//             *fri_alpha,
//         );
//         let log_n = common_data.degree_bits + common_data.config.fri_config.rate_bits;
//         // Simulate the proof verification and collect the inferred elements.
//         // The content of the loop is basically the same as the `fri_verifier_query_round` function.
//         for &(mut x_index) in fri_query_indices {
//             let mut subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
//                 * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(x_index, log_n) as u64);
//             let mut old_eval = fri_combine_initial::<F, C, D>(
//                 &common_data.get_fri_instance(*plonk_zeta),
//                 &self
//                     .proof
//                     .opening_proof
//                     .query_round_proofs
//                     .initial_trees_proofs[&x_index],
//                 *fri_alpha,
//                 subgroup_x,
//                 &precomputed_reduced_evals,
//                 &common_data.fri_params,
//             );
//             for (i, &arity_bits) in common_data
//                 .fri_params
//                 .reduction_arity_bits
//                 .iter()
//                 .enumerate()
//             {
//                 let coset_index = x_index >> arity_bits;
//                 if !seen_indices_by_depth[i].insert(coset_index) {
//                     // If this index has already been seen, we can skip the rest of the reductions.
//                     break;
//                 }
//                 fri_inferred_elements.push(old_eval);
//                 let arity = 1 << arity_bits;
//                 let mut evals = self.proof.opening_proof.query_round_proofs.steps[i][&coset_index]
//                     .evals
//                     .clone();
//                 let x_index_within_coset = x_index & (arity - 1);
//                 evals.insert(x_index_within_coset, old_eval);
//                 old_eval = compute_evaluation(
//                     subgroup_x,
//                     x_index_within_coset,
//                     arity_bits,
//                     &evals,
//                     fri_betas[i],
//                 );
//                 subgroup_x = subgroup_x.exp_power_of_2(arity_bits);
//                 x_index = coset_index;
//             }
//         }
//         FriInferredElements(fri_inferred_elements)
//     }
// }
