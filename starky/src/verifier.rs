use anyhow::{ensure, Result};
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_data::CommonCircuitData;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2_util::log2_strict;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::proof::{StarkOpeningSet, StarkProof, StarkProofChallenges, StarkProofWithPublicInputs};
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

pub(crate) fn verify<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    proof_with_pis: StarkProofWithPublicInputs<F, C, D>,
    config: &StarkConfig,
) -> Result<()> {
    let challenges = proof_with_pis.get_challenges(config)?;
    verify_with_challenges(proof_with_pis, challenges, verifier_data, common_data)
}

pub(crate) fn verify_with_challenges<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    stark: S,
    proof_with_pis: StarkProofWithPublicInputs<F, C, D>,
    challenges: StarkProofChallenges<F, D>,
    config: &StarkConfig,
) -> Result<()> {
    let StarkProofWithPublicInputs {
        proof,
        public_inputs,
    } = proof_with_pis;
    let degree = recover_degree(&proof, config);
    let degree_log = log2_strict(degree);

    let local_constants = &proof.openings.constants;
    let local_values = &proof.openings.local_values;
    let next_values = &proof.openings.local_values;
    let StarkOpeningSet {
        local_values,
        next_values,
        permutation_zs,
        quotient_polys,
    } = &proof.openings;
    let vars = StarkEvaluationVars {
        local_values,
        next_values,
        public_inputs: &public_inputs,
    };

    let mut consumer = ConstraintConsumer::<F::Extension>::new(
        challenges.stark_alphas,
        lagrange_first.values[i],
        lagrange_last.values[i],
    );
    let (l_1, l_n) = eval_l_1_and_l_last(degree_log, challenges.stark_zeta);
    stark.eval_ext()

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let quotient_polys_zeta = &proof.openings.quotient_polys;
    let zeta_pow_deg = challenges
        .plonk_zeta
        .exp_power_of_2(common_data.degree_bits);
    let z_h_zeta = zeta_pow_deg - F::Extension::ONE;
    // `quotient_polys_zeta` holds `num_challenges * quotient_degree_factor` evaluations.
    // Each chunk of `quotient_degree_factor` holds the evaluations of `t_0(zeta),...,t_{quotient_degree_factor-1}(zeta)`
    // where the "real" quotient polynomial is `t(X) = t_0(X) + t_1(X)*X^n + t_2(X)*X^{2n} + ...`.
    // So to reconstruct `t(zeta)` we can compute `reduce_with_powers(chunk, zeta^n)` for each
    // `quotient_degree_factor`-sized chunk of the original evaluations.
    for (i, chunk) in quotient_polys_zeta
        .chunks(common_data.quotient_degree_factor)
        .enumerate()
    {
        ensure!(vanishing_polys_zeta[i] == z_h_zeta * reduce_with_powers(chunk, zeta_pow_deg));
    }

    let merkle_caps = &[
        verifier_data.constants_sigmas_cap.clone(),
        proof.wires_cap,
        proof.plonk_zs_partial_products_cap,
        proof.quotient_polys_cap,
    ];

    verify_fri_proof::<F, C, D>(
        &common_data.get_fri_instance(challenges.plonk_zeta),
        &proof.openings,
        &challenges,
        merkle_caps,
        &proof.opening_proof,
        &common_data.fri_params,
    )?;

    Ok(())
}

/// Evaluate the Lagrange basis `L_1` and `L_n` at a point `x`.
fn eval_l_1_and_l_last<F: Field>(log_n: usize, x: F) -> (F,F) {
    let n = 1 << log_n;
    let g = F::primitive_root_of_unity(log_n);
    let z_x = x.exp_power_of_2(log_n);
    let invs = F::batch_multiplicative_inverse(&[F::from_canonical_usize(n) * (x - F::ONE), F::from_canonical_usize(n) * (g*x - F::ONE)]);

    (z_x * invs[0], z_x * invs[1])
}

fn recover_degree<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(proof: &StarkProof<F, C,D>, config: &StarkConfig) -> usize {
    1<<(proof.opening_proof.query_round_proofs[0].initial_trees_proof.evals_proofs[0].1.siblings.len() + config.fri_config.cap_height)
}