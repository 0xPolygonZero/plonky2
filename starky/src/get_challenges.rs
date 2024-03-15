use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialCoeffs;
use plonky2::fri::proof::{FriProof, FriProofTarget};
use plonky2::gadgets::polynomial::PolynomialCoeffsExtTarget;
use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};

use crate::config::StarkConfig;
use crate::lookup::{
    get_grand_product_challenge_set, get_grand_product_challenge_set_target,
    GrandProductChallengeSet,
};
use crate::proof::*;

/// Generates challenges for a STARK proof from a challenger and given
/// all the arguments needed to update the challenger state.
///
/// Note: `trace_cap` is passed as `Option` to signify whether to observe it
/// or not by the challenger. Observing it here could be redundant in a
/// multi-STARK system where trace caps would have already been observed
/// before proving individually each STARK.
fn get_challenges<F, C, const D: usize>(
    challenger: &mut Challenger<F, C::Hasher>,
    challenges: Option<&GrandProductChallengeSet<F>>,
    trace_cap: Option<&MerkleCap<F, C::Hasher>>,
    auxiliary_polys_cap: Option<&MerkleCap<F, C::Hasher>>,
    quotient_polys_cap: Option<&MerkleCap<F, C::Hasher>>,
    openings: &StarkOpeningSet<F, D>,
    commit_phase_merkle_caps: &[MerkleCap<F, C::Hasher>],
    final_poly: &PolynomialCoeffs<F::Extension>,
    pow_witness: F,
    config: &StarkConfig,
    degree_bits: usize,
) -> StarkProofChallenges<F, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let num_challenges = config.num_challenges;

    if let Some(cap) = &trace_cap {
        challenger.observe_cap(cap);
    }

    let lookup_challenge_set = if let Some(&challenges) = challenges.as_ref() {
        Some(challenges.clone())
    } else {
        auxiliary_polys_cap
            .is_some()
            .then(|| get_grand_product_challenge_set(challenger, num_challenges))
    };

    if let Some(cap) = &auxiliary_polys_cap {
        challenger.observe_cap(cap);
    }

    let stark_alphas = challenger.get_n_challenges(num_challenges);

    if let Some(quotient_polys_cap) = quotient_polys_cap {
        challenger.observe_cap(quotient_polys_cap);
    }
    let stark_zeta = challenger.get_extension_challenge::<D>();

    challenger.observe_openings(&openings.to_fri_openings());

    StarkProofChallenges {
        lookup_challenge_set,
        stark_alphas,
        stark_zeta,
        fri_challenges: challenger.fri_challenges::<C, D>(
            commit_phase_merkle_caps,
            final_poly,
            pow_witness,
            degree_bits,
            &config.fri_config,
        ),
    }
}

impl<F, C, const D: usize> StarkProof<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges(
        &self,
        challenger: &mut Challenger<F, C::Hasher>,
        challenges: Option<&GrandProductChallengeSet<F>>,
        ignore_trace_cap: bool,
        config: &StarkConfig,
    ) -> StarkProofChallenges<F, D> {
        let degree_bits = self.recover_degree_bits(config);

        let StarkProof {
            trace_cap,
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProof {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
        } = &self;

        let trace_cap = if ignore_trace_cap {
            None
        } else {
            Some(trace_cap)
        };

        get_challenges::<F, C, D>(
            challenger,
            challenges,
            trace_cap,
            auxiliary_polys_cap.as_ref(),
            quotient_polys_cap.as_ref(),
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            config,
            degree_bits,
        )
    }
}

impl<F, C, const D: usize> StarkProofWithPublicInputs<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges(
        &self,
        challenger: &mut Challenger<F, C::Hasher>,
        challenges: Option<&GrandProductChallengeSet<F>>,
        ignore_trace_cap: bool,
        config: &StarkConfig,
    ) -> StarkProofChallenges<F, D> {
        self.proof
            .get_challenges(challenger, challenges, ignore_trace_cap, config)
    }
}

/// Circuit version of `get_challenges`, with the same flexibility around
/// `trace_cap` being passed as an `Option`.
fn get_challenges_target<F, C, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    challenges: Option<&GrandProductChallengeSet<Target>>,
    trace_cap: Option<&MerkleCapTarget>,
    auxiliary_polys_cap: Option<&MerkleCapTarget>,
    quotient_polys_cap: Option<&MerkleCapTarget>,
    openings: &StarkOpeningSetTarget<D>,
    commit_phase_merkle_caps: &[MerkleCapTarget],
    final_poly: &PolynomialCoeffsExtTarget<D>,
    pow_witness: Target,
    config: &StarkConfig,
) -> StarkProofChallengesTarget<D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    let num_challenges = config.num_challenges;

    if let Some(trace_cap) = trace_cap {
        challenger.observe_cap(trace_cap);
    }

    let lookup_challenge_set = if let Some(&challenges) = challenges.as_ref() {
        Some(challenges.clone())
    } else {
        auxiliary_polys_cap
            .is_some()
            .then(|| get_grand_product_challenge_set_target(builder, challenger, num_challenges))
    };

    if let Some(cap) = auxiliary_polys_cap {
        challenger.observe_cap(cap);
    }

    let stark_alphas = challenger.get_n_challenges(builder, num_challenges);

    if let Some(cap) = quotient_polys_cap {
        challenger.observe_cap(cap);
    }

    let stark_zeta = challenger.get_extension_challenge(builder);

    challenger.observe_openings(&openings.to_fri_openings(builder.zero()));

    StarkProofChallengesTarget {
        lookup_challenge_set,
        stark_alphas,
        stark_zeta,
        fri_challenges: challenger.fri_challenges(
            builder,
            commit_phase_merkle_caps,
            final_poly,
            pow_witness,
            &config.fri_config,
        ),
    }
}

impl<const D: usize> StarkProofTarget<D> {
    /// Creates all Fiat-Shamir `Target` challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges<F, C>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
        challenges: Option<&GrandProductChallengeSet<Target>>,
        ignore_trace_cap: bool,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        C::Hasher: AlgebraicHasher<F>,
    {
        let StarkProofTarget {
            trace_cap,
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProofTarget {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
        } = self;

        let trace_cap = if ignore_trace_cap {
            None
        } else {
            Some(trace_cap)
        };

        get_challenges_target::<F, C, D>(
            builder,
            challenger,
            challenges,
            trace_cap,
            auxiliary_polys_cap.as_ref(),
            quotient_polys_cap.as_ref(),
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            config,
        )
    }
}

impl<const D: usize> StarkProofWithPublicInputsTarget<D> {
    /// Creates all Fiat-Shamir `Target` challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges<F, C>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
        challenges: Option<&GrandProductChallengeSet<Target>>,
        ignore_trace_cap: bool,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        C::Hasher: AlgebraicHasher<F>,
    {
        self.proof
            .get_challenges::<F, C>(builder, challenger, challenges, ignore_trace_cap, config)
    }
}

// TODO: Deal with the compressed stuff.
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
