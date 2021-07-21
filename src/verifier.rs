use anyhow::{ensure, Result};

use crate::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::hash::hash_n_to_hash;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::reduce_with_powers;
use crate::proof::ProofWithPublicInputs;
use crate::vanishing_poly::eval_vanishing_poly;
use crate::vars::EvaluationVars;

pub(crate) fn verify<F: Extendable<D>, const D: usize>(
    proof_with_pis: ProofWithPublicInputs<F, D>,
    verifier_data: &VerifierOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let ProofWithPublicInputs {
        proof,
        public_inputs,
    } = proof_with_pis;
    let config = &common_data.config;
    let num_challenges = config.num_challenges;

    let mut challenger = Challenger::new();

    // Observe the instance.
    challenger.observe_hash(&common_data.circuit_digest);
    challenger.observe_elements(&public_inputs);

    challenger.observe_hash(&proof.wires_root);
    let betas = challenger.get_n_challenges(num_challenges);
    let gammas = challenger.get_n_challenges(num_challenges);

    challenger.observe_hash(&proof.plonk_zs_partial_products_root);
    let alphas = challenger.get_n_challenges(num_challenges);

    challenger.observe_hash(&proof.quotient_polys_root);
    let zeta = challenger.get_extension_challenge();

    let local_constants = &proof.openings.constants;
    let local_wires = &proof.openings.wires;
    let public_inputs_hash = &hash_n_to_hash(public_inputs, true);
    let vars = EvaluationVars {
        local_constants,
        local_wires,
        public_inputs_hash,
    };
    let local_zs = &proof.openings.plonk_zs;
    let next_zs = &proof.openings.plonk_zs_right;
    let s_sigmas = &proof.openings.plonk_sigmas;
    let partial_products = &proof.openings.partial_products;

    // Evaluate the vanishing polynomial at our challenge point, zeta.
    let vanishing_polys_zeta = eval_vanishing_poly(
        common_data,
        zeta,
        vars,
        local_zs,
        next_zs,
        partial_products,
        s_sigmas,
        &betas,
        &gammas,
        &alphas,
    );

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let quotient_polys_zeta = &proof.openings.quotient_polys;
    let zeta_pow_deg = zeta.exp_power_of_2(common_data.degree_bits);
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

    let evaluations = proof.openings.clone();

    let merkle_roots = &[
        verifier_data.constants_sigmas_root,
        proof.wires_root,
        proof.plonk_zs_partial_products_root,
        proof.quotient_polys_root,
    ];

    proof.opening_proof.verify(
        zeta,
        &evaluations,
        merkle_roots,
        &mut challenger,
        common_data,
    )?;

    Ok(())
}
