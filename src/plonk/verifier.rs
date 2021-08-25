use anyhow::{ensure, Result};

use crate::field::extension_field::Extendable;
use crate::field::field_types::Field;
use crate::fri::verifier::{verify_compressed_fri_proof, verify_fri_proof};
use crate::hash::hashing::hash_n_to_hash;
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::challenger::Challenger;
use crate::plonk::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::plonk::plonk_common::reduce_with_powers;
use crate::plonk::proof::{
    CompressedProof, CompressedProofWithPublicInputs, OpeningSet, Proof, ProofWithPublicInputs,
};
use crate::plonk::vanishing_poly::eval_vanishing_poly;
use crate::plonk::vars::EvaluationVars;

fn verify_iop<F: Extendable<D>, const D: usize>(
    public_inputs: Vec<F>,
    wires_cap: MerkleCap<F>,
    plonk_zs_partial_products_cap: MerkleCap<F>,
    quotient_polys_cap: MerkleCap<F>,
    openings: &OpeningSet<F, D>,
    challenger: &mut Challenger<F>,
    verifier_data: &VerifierOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<(F::Extension, [MerkleCap<F>; 4])> {
    let config = &common_data.config;
    let num_challenges = config.num_challenges;

    let public_inputs_hash = &hash_n_to_hash(public_inputs, true);

    // Observe the instance.
    challenger.observe_hash(&common_data.circuit_digest);
    challenger.observe_hash(&public_inputs_hash);

    challenger.observe_cap(&wires_cap);
    let betas = challenger.get_n_challenges(num_challenges);
    let gammas = challenger.get_n_challenges(num_challenges);

    challenger.observe_cap(&plonk_zs_partial_products_cap);
    let alphas = challenger.get_n_challenges(num_challenges);

    challenger.observe_cap(&quotient_polys_cap);
    let zeta = challenger.get_extension_challenge();

    let local_constants = &openings.constants;
    let local_wires = &openings.wires;
    let vars = EvaluationVars {
        local_constants,
        local_wires,
        public_inputs_hash,
    };
    let local_zs = &openings.plonk_zs;
    let next_zs = &openings.plonk_zs_right;
    let s_sigmas = &openings.plonk_sigmas;
    let partial_products = &openings.partial_products;

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
    let quotient_polys_zeta = &openings.quotient_polys;
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

    Ok((
        zeta,
        [
            verifier_data.constants_sigmas_cap.clone(),
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
        ],
    ))
}

pub(crate) fn verify<F: Extendable<D>, const D: usize>(
    proof_with_pis: ProofWithPublicInputs<F, D>,
    verifier_data: &VerifierOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let mut challenger = Challenger::new();
    let ProofWithPublicInputs {
        proof:
            Proof {
                wires_cap,
                plonk_zs_partial_products_cap,
                quotient_polys_cap,
                openings,
                opening_proof,
            },
        public_inputs,
    } = proof_with_pis;

    let (zeta, merkle_caps) = verify_iop(
        public_inputs,
        wires_cap,
        plonk_zs_partial_products_cap,
        quotient_polys_cap,
        &openings,
        &mut challenger,
        verifier_data,
        common_data,
    )?;

    verify_fri_proof(
        &openings,
        zeta,
        &merkle_caps,
        opening_proof,
        &mut challenger,
        &common_data,
    )?;

    Ok(())
}

pub(crate) fn verify_compressed<F: Extendable<D>, const D: usize>(
    proof_with_pis: CompressedProofWithPublicInputs<F, D>,
    verifier_data: &VerifierOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let mut challenger = Challenger::new();
    let CompressedProofWithPublicInputs {
        proof:
            CompressedProof {
                wires_cap,
                plonk_zs_partial_products_cap,
                quotient_polys_cap,
                openings,
                opening_proof,
            },
        public_inputs,
    } = proof_with_pis;

    let (zeta, merkle_caps) = verify_iop(
        public_inputs,
        wires_cap,
        plonk_zs_partial_products_cap,
        quotient_polys_cap,
        &openings,
        &mut challenger,
        verifier_data,
        common_data,
    )?;

    verify_compressed_fri_proof(
        &openings,
        zeta,
        &merkle_caps,
        opening_proof,
        &mut challenger,
        &common_data,
    )?;

    Ok(())
}
