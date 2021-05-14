use std::time::Instant;

use log::info;
use rayon::prelude::*;

use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::fft::ifft;
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::{eval_l_1, evaluate_gate_constraints, reduce_with_powers_multi};
use crate::polynomial::commitment::ListPolynomialCommitment;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::Proof;
use crate::util::transpose;
use crate::vars::EvaluationVars;
use crate::wire::Wire;
use crate::witness::PartialWitness;

macro_rules! timed {
    ($a:expr, $msg:expr) => {{
        let timer = Instant::now();
        let res = $a;
        info!("{:.3}s {}", timer.elapsed().as_secs_f32(), $msg);
        res
    }};
}

/// Corresponds to constants - sigmas - wires - zs - quotient — polynomial commitments.
pub const PLONK_BLINDING: [bool; 5] = [false, false, true, true, true];

pub(crate) fn prove<F: Field>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
    inputs: PartialWitness<F>,
) -> Proof<F> {
    let fri_config = &common_data.config.fri_config;

    let start_proof_gen = Instant::now();

    let mut witness = inputs;
    info!("Running {} generators", prover_data.generators.len());
    timed!(
        generate_partial_witness(&mut witness, &prover_data.generators),
        "to generate witness"
    );

    let config = &common_data.config;
    let num_wires = config.num_wires;
    let num_checks = config.num_checks;
    let quotient_degree = common_data.quotient_degree();

    let degree = common_data.degree();
    let wires_polynomials: Vec<PolynomialCoeffs<F>> = timed!(
        (0..num_wires)
            .into_par_iter()
            .map(|i| compute_wire_polynomial(i, &witness, degree))
            .collect(),
        "to compute wire polynomials"
    );

    // TODO: Could try parallelizing the transpose, or not doing it explicitly, instead having
    // merkle_root_bit_rev_order do it implicitly.
    let wires_commitment = timed!(
        ListPolynomialCommitment::new(wires_polynomials, fri_config.rate_bits, true),
        "to compute wires commitment"
    );

    let mut challenger = Challenger::new();
    // Observe the instance.
    // TODO: Need to include public inputs as well.
    challenger.observe_hash(&common_data.circuit_digest);

    challenger.observe_hash(&wires_commitment.merkle_tree.root);
    let betas = challenger.get_n_challenges(num_checks);
    let gammas = challenger.get_n_challenges(num_checks);

    let plonk_z_vecs = timed!(compute_zs(&common_data), "to compute Z's");

    let plonk_zs_commitment = timed!(
        ListPolynomialCommitment::new(plonk_z_vecs, fri_config.rate_bits, true),
        "to commit to Z's"
    );

    challenger.observe_hash(&plonk_zs_commitment.merkle_tree.root);

    let alphas = challenger.get_n_challenges(num_checks);

    let vanishing_polys = timed!(
        compute_vanishing_polys(
            common_data,
            prover_data,
            &wires_commitment,
            &plonk_zs_commitment,
            &betas,
            &gammas,
            &alphas,
        ),
        "to compute vanishing polys"
    );

    // Compute the quotient polynomials, aka `t` in the Plonk paper.
    let quotient_polys_commitment = timed!(
        {
            let mut all_quotient_poly_chunks = Vec::with_capacity(num_checks * quotient_degree);
            for vanishing_poly in vanishing_polys.into_iter() {
                let vanishing_poly_coeff = ifft(vanishing_poly);
                let quotient_poly_coeff = vanishing_poly_coeff.divide_by_z_h(degree);
                // Split t into degree-n chunks.
                let quotient_poly_coeff_chunks = quotient_poly_coeff.chunks(degree);
                all_quotient_poly_chunks.extend(quotient_poly_coeff_chunks);
            }
            ListPolynomialCommitment::new(all_quotient_poly_chunks, fri_config.rate_bits, true)
        },
        "to compute quotient polys and commit to them"
    );

    challenger.observe_hash(&quotient_polys_commitment.merkle_tree.root);

    // TODO: How many do we need?
    let num_zetas = 2;
    let zetas = challenger.get_n_challenges(num_zetas);

    let (opening_proof, openings) = timed!(
        ListPolynomialCommitment::batch_open_plonk(
            &[
                &prover_data.constants_commitment,
                &prover_data.sigmas_commitment,
                &wires_commitment,
                &plonk_zs_commitment,
                &quotient_polys_commitment,
            ],
            &zetas,
            &mut challenger,
            &common_data.config.fri_config
        ),
        "to compute opening proofs"
    );

    info!(
        "{:.3}s for overall witness & proof generation",
        start_proof_gen.elapsed().as_secs_f32()
    );

    Proof {
        wires_root: wires_commitment.merkle_tree.root,
        plonk_zs_root: plonk_zs_commitment.merkle_tree.root,
        quotient_polys_root: quotient_polys_commitment.merkle_tree.root,
        openings,
        opening_proof,
    }
}

fn compute_zs<F: Field>(common_data: &CommonCircuitData<F>) -> Vec<PolynomialCoeffs<F>> {
    (0..common_data.config.num_checks)
        .map(|i| compute_z(common_data, i))
        .collect()
}

fn compute_z<F: Field>(common_data: &CommonCircuitData<F>, _i: usize) -> PolynomialCoeffs<F> {
    PolynomialCoeffs::zero(common_data.degree()) // TODO
}

// TODO: Parallelize.
fn compute_vanishing_polys<F: Field>(
    common_data: &CommonCircuitData<F>,
    prover_data: &ProverOnlyCircuitData<F>,
    wires_commitment: &ListPolynomialCommitment<F>,
    plonk_zs_commitment: &ListPolynomialCommitment<F>,
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<PolynomialValues<F>> {
    let lde_size = common_data.lde_size();
    let lde_gen = common_data.lde_generator();
    let num_checks = common_data.config.num_checks;

    let points = F::cyclic_subgroup_known_order(lde_gen, lde_size);
    let values: Vec<Vec<F>> = points
        .into_par_iter()
        .enumerate()
        .map(|(i, x)| {
            let i_next = (i + 1) % lde_size;
            let local_wires = wires_commitment.leaf(i);
            let local_constants = prover_data.constants_commitment.leaf(i);
            let local_plonk_zs = plonk_zs_commitment.leaf(i);
            let next_plonk_zs = plonk_zs_commitment.leaf(i_next);
            let s_sigmas = prover_data.sigmas_commitment.leaf(i);

            debug_assert_eq!(local_wires.len(), common_data.config.num_wires);
            debug_assert_eq!(local_plonk_zs.len(), num_checks);

            let vars = EvaluationVars {
                local_constants,
                local_wires,
            };
            compute_vanishing_poly_entry(
                common_data,
                x,
                vars,
                local_plonk_zs,
                next_plonk_zs,
                s_sigmas,
                betas,
                gammas,
                alphas,
            )
        })
        .collect();

    transpose(&values)
        .into_iter()
        .map(PolynomialValues::new)
        .collect()
}

/// Evaluate the vanishing polynomial at `x`. In this context, the vanishing polynomial is a random
/// linear combination of gate constraints, plus some other terms relating to the permutation
/// argument. All such terms should vanish on `H`.
fn compute_vanishing_poly_entry<F: Field>(
    common_data: &CommonCircuitData<F>,
    x: F,
    vars: EvaluationVars<F>,
    local_plonk_zs: &[F],
    next_plonk_zs: &[F],
    s_sigmas: &[F],
    betas: &[F],
    gammas: &[F],
    alphas: &[F],
) -> Vec<F> {
    let constraint_terms =
        evaluate_gate_constraints(&common_data.gates, common_data.num_gate_constraints, vars);

    // The L_1(x) (Z(x) - 1) vanishing terms.
    let mut vanishing_z_1_terms = Vec::new();
    // The Z(x) f'(x) - g'(x) Z(g x) terms.
    let mut vanishing_v_shift_terms = Vec::new();

    for i in 0..common_data.config.num_checks {
        let z_x = local_plonk_zs[i];
        let z_gz = next_plonk_zs[i];
        vanishing_z_1_terms.push(eval_l_1(common_data.degree(), x) * (z_x - F::ONE));

        let mut f_prime = F::ONE;
        let mut g_prime = F::ONE;
        for j in 0..common_data.config.num_routed_wires {
            let wire_value = vars.local_wires[j];
            let k_i = common_data.k_is[j];
            let s_id = k_i * x;
            let s_sigma = s_sigmas[j];
            f_prime *= wire_value + betas[i] * s_id + gammas[i];
            g_prime *= wire_value + betas[i] * s_sigma + gammas[i];
        }
        vanishing_v_shift_terms.push(f_prime * z_x - g_prime * z_gz);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ]
    .concat();

    reduce_with_powers_multi(&vanishing_terms, alphas)
}

fn compute_wire_polynomial<F: Field>(
    input: usize,
    witness: &PartialWitness<F>,
    degree: usize,
) -> PolynomialCoeffs<F> {
    let wire_values = (0..degree)
        // Some gates do not use all wires, and we do not require that generators populate unused
        // wires, so some wire values will not be set. We can set these to any value; here we
        // arbitrary pick zero. Ideally we would verify that no constraints operate on these unset
        // wires, but that isn't trivial.
        .map(|gate| {
            witness
                .try_get_wire(Wire { gate, input })
                .unwrap_or(F::ZERO)
        })
        .collect();
    PolynomialValues::new(wire_values).ifft()
}
