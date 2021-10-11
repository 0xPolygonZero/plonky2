use anyhow::{ensure, Result};

use crate::field::extension_field::{flatten, Extendable, FieldExtension};
use crate::field::field_types::{Field, RichField};
use crate::field::interpolation::{barycentric_weights, interpolate, interpolate2};
use crate::fri::proof::{FriInitialTreeProof, FriProof, FriQueryRound};
use crate::fri::FriConfig;
use crate::hash::merkle_proofs::verify_merkle_proof;
use crate::hash::merkle_tree::MerkleCap;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::plonk::proof::{OpeningSet, ProofChallenges};
use crate::util::reducing::ReducingFactor;
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};

/// Computes P'(x^arity) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity
/// and P' is the FRI reduced polynomial.
pub(crate) fn compute_evaluation<F: Field + Extendable<D>, const D: usize>(
    x: F,
    x_index_within_coset: usize,
    arity_bits: usize,
    evals: &[F::Extension],
    beta: F::Extension,
) -> F::Extension {
    let arity = 1 << arity_bits;
    debug_assert_eq!(evals.len(), arity);

    let g = F::primitive_root_of_unity(arity_bits);

    // The evaluation vector needs to be reordered first.
    let mut evals = evals.to_vec();
    reverse_index_bits_in_place(&mut evals);
    let rev_x_index_within_coset = reverse_bits(x_index_within_coset, arity_bits);
    let coset_start = x * g.exp_u64((arity - rev_x_index_within_coset) as u64);
    // The answer is gotten by interpolating {(x*g^i, P(x*g^i))} and evaluating at beta.
    let points = g
        .powers()
        .map(|y| (coset_start * y).into())
        .zip(evals)
        .collect::<Vec<_>>();
    let barycentric_weights = barycentric_weights(&points);
    interpolate(&points, beta, &barycentric_weights)
}

pub(crate) fn fri_verify_proof_of_work<F: RichField + Extendable<D>, const D: usize>(
    fri_pow_response: F,
    config: &FriConfig,
) -> Result<()> {
    ensure!(
        fri_pow_response.to_canonical_u64().leading_zeros()
            >= config.proof_of_work_bits + (64 - F::order().bits()) as u32,
        "Invalid proof of work witness."
    );

    Ok(())
}

pub(crate) fn verify_fri_proof<F: RichField + Extendable<D>, const D: usize>(
    // Openings of the PLONK polynomials.
    os: &OpeningSet<F, D>,
    challenges: &ProofChallenges<F, D>,
    initial_merkle_caps: &[MerkleCap<F>],
    proof: &FriProof<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let config = &common_data.config;
    ensure!(
        common_data.final_poly_len() == proof.final_poly.len(),
        "Final polynomial has wrong degree."
    );

    // Size of the LDE domain.
    let n = common_data.lde_size();

    // Check PoW.
    fri_verify_proof_of_work(challenges.fri_pow_response, &config.fri_config)?;

    // Check that parameters are coherent.
    ensure!(
        config.fri_config.num_query_rounds == proof.query_round_proofs.len(),
        "Number of query rounds does not match config."
    );

    let precomputed_reduced_evals =
        PrecomputedReducedEvals::from_os_and_alpha(os, challenges.fri_alpha);
    for (round, (&x_index, round_proof)) in challenges
        .fri_query_indices
        .iter()
        .zip(&proof.query_round_proofs)
        .enumerate()
    {
        fri_verifier_query_round(
            challenges,
            precomputed_reduced_evals,
            initial_merkle_caps,
            &proof,
            x_index,
            n,
            round,
            round_proof,
            common_data,
        )?;
    }

    Ok(())
}

fn fri_verify_initial_proof<F: RichField>(
    x_index: usize,
    proof: &FriInitialTreeProof<F>,
    initial_merkle_caps: &[MerkleCap<F>],
) -> Result<()> {
    for ((evals, merkle_proof), cap) in proof.evals_proofs.iter().zip(initial_merkle_caps) {
        verify_merkle_proof(evals.clone(), x_index, cap, merkle_proof)?;
    }

    Ok(())
}

/// Holds the reduced (by `alpha`) evaluations at `zeta` for the polynomial opened just at
/// zeta, for `Z` at zeta and for `Z` at `g*zeta`.
#[derive(Copy, Clone, Debug)]
pub(crate) struct PrecomputedReducedEvals<F: Extendable<D>, const D: usize> {
    pub single: F::Extension,
    pub zs: F::Extension,
    pub zs_right: F::Extension,
}

impl<F: Extendable<D>, const D: usize> PrecomputedReducedEvals<F, D> {
    pub(crate) fn from_os_and_alpha(os: &OpeningSet<F, D>, alpha: F::Extension) -> Self {
        let mut alpha = ReducingFactor::new(alpha);
        let single = alpha.reduce(
            os.constants
                .iter()
                .chain(&os.plonk_sigmas)
                .chain(&os.wires)
                .chain(&os.quotient_polys)
                .chain(&os.partial_products),
        );
        let zs = alpha.reduce(os.plonk_zs.iter());
        let zs_right = alpha.reduce(os.plonk_zs_right.iter());

        Self {
            single,
            zs,
            zs_right,
        }
    }
}

pub(crate) fn fri_combine_initial<F: RichField + Extendable<D>, const D: usize>(
    proof: &FriInitialTreeProof<F>,
    alpha: F::Extension,
    zeta: F::Extension,
    subgroup_x: F,
    precomputed_reduced_evals: PrecomputedReducedEvals<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> F::Extension {
    let config = &common_data.config;
    assert!(D > 1, "Not implemented for D=1.");
    let degree_log = common_data.degree_bits;
    // debug_assert_eq!(
    //     degree_log,
    //     common_data.config.cap_height + proof.evals_proofs[0].1.siblings.len() - config.rate_bits
    // );
    let subgroup_x = F::Extension::from_basefield(subgroup_x);
    let mut alpha = ReducingFactor::new(alpha);
    let mut sum = F::Extension::ZERO;

    // We will add three terms to `sum`:
    // - one for various polynomials which are opened at a single point `x`
    // - one for Zs, which are opened at `x` and `g x`

    // Polynomials opened at `x`, i.e., the constants-sigmas, wires, quotient and partial products polynomials.
    let single_evals = [
        PlonkPolynomials::CONSTANTS_SIGMAS,
        PlonkPolynomials::WIRES,
        PlonkPolynomials::QUOTIENT,
    ]
    .iter()
    .flat_map(|&p| proof.unsalted_evals(p, config.zero_knowledge))
    .chain(
        &proof.unsalted_evals(PlonkPolynomials::ZS_PARTIAL_PRODUCTS, config.zero_knowledge)
            [common_data.partial_products_range()],
    )
    .map(|&e| F::Extension::from_basefield(e));
    let single_composition_eval = alpha.reduce(single_evals);
    let single_numerator = single_composition_eval - precomputed_reduced_evals.single;
    let single_denominator = subgroup_x - zeta;
    sum += single_numerator / single_denominator;
    alpha.reset();

    // Polynomials opened at `x` and `g x`, i.e., the Zs polynomials.
    let zs_evals = proof
        .unsalted_evals(PlonkPolynomials::ZS_PARTIAL_PRODUCTS, config.zero_knowledge)
        .iter()
        .map(|&e| F::Extension::from_basefield(e))
        .take(common_data.zs_range().end);
    let zs_composition_eval = alpha.reduce(zs_evals);
    let zeta_right = F::Extension::primitive_root_of_unity(degree_log) * zeta;
    let zs_interpol = interpolate2(
        [
            (zeta, precomputed_reduced_evals.zs),
            (zeta_right, precomputed_reduced_evals.zs_right),
        ],
        subgroup_x,
    );
    let zs_numerator = zs_composition_eval - zs_interpol;
    let zs_denominator = (subgroup_x - zeta) * (subgroup_x - zeta_right);
    sum = alpha.shift(sum);
    sum += zs_numerator / zs_denominator;

    sum
}

fn fri_verifier_query_round<F: RichField + Extendable<D>, const D: usize>(
    challenges: &ProofChallenges<F, D>,
    precomputed_reduced_evals: PrecomputedReducedEvals<F, D>,
    initial_merkle_caps: &[MerkleCap<F>],
    proof: &FriProof<F, D>,
    mut x_index: usize,
    n: usize,
    round: usize,
    round_proof: &FriQueryRound<F, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    fri_verify_initial_proof(
        x_index,
        &round_proof.initial_trees_proof,
        initial_merkle_caps,
    )?;
    // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
    let log_n = log2_strict(n);
    let mut subgroup_x = F::MULTIPLICATIVE_GROUP_GENERATOR
        * F::primitive_root_of_unity(log_n).exp_u64(reverse_bits(x_index, log_n) as u64);

    // old_eval is the last derived evaluation; it will be checked for consistency with its
    // committed "parent" value in the next iteration.
    let mut old_eval = fri_combine_initial(
        &round_proof.initial_trees_proof,
        challenges.fri_alpha,
        challenges.plonk_zeta,
        subgroup_x,
        precomputed_reduced_evals,
        common_data,
    );

    for (i, &arity_bits) in common_data
        .fri_params
        .reduction_arity_bits
        .iter()
        .enumerate()
    {
        let arity = 1 << arity_bits;
        let evals = &round_proof.steps[i].evals;

        // Split x_index into the index of the coset x is in, and the index of x within that coset.
        let coset_index = x_index >> arity_bits;
        let x_index_within_coset = x_index & (arity - 1);

        // Check consistency with our old evaluation from the previous round.
        ensure!(evals[x_index_within_coset] == old_eval);

        // Infer P(y) from {P(x)}_{x^arity=y}.
        old_eval = compute_evaluation(
            subgroup_x,
            x_index_within_coset,
            arity_bits,
            evals,
            challenges.fri_betas[i],
        );

        verify_merkle_proof(
            flatten(evals),
            coset_index,
            &proof.commit_phase_merkle_caps[i],
            &round_proof.steps[i].merkle_proof,
        )?;

        // Update the point x to x^arity.
        subgroup_x = subgroup_x.exp_power_of_2(arity_bits);

        x_index = coset_index;
    }

    // Final check of FRI. After all the reductions, we check that the final polynomial is equal
    // to the one sent by the prover.
    ensure!(
        proof.final_poly.eval(subgroup_x.into()) == old_eval,
        "Final polynomial evaluation is invalid."
    );

    Ok(())
}
