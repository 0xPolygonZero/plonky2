use std::time::Instant;

use log::info;
use rayon::prelude::*;

use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::constraint_polynomial::EvaluationVars;
use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::hash::merkle_root_bit_rev_order;
use crate::plonk_challenger::Challenger;
use crate::plonk_common::{eval_l_1, reduce_with_powers_multi, evaluate_gate_constraints};
use crate::polynomial::division::divide_by_z_h;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::Proof;
use crate::util::{transpose, transpose_poly_values};
use crate::wire::Wire;
use crate::witness::PartialWitness;

pub(crate) fn prove<F: Field>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
    inputs: PartialWitness<F>,
) -> Proof<F> {
    let start_proof_gen = Instant::now();

    let start_witness = Instant::now();
    let mut witness = inputs;
    info!("Running {} generators", prover_data.generators.len());
    generate_partial_witness(&mut witness, &prover_data.generators);
    info!("{:.3}s to generate witness",
          start_witness.elapsed().as_secs_f32());

    let config = common_data.config;
    let num_wires = config.num_wires;
    let num_checks = config.num_checks;
    let quotient_degree = common_data.quotient_degree();

    let start_wire_ldes = Instant::now();
    let degree = common_data.degree();
    let wire_ldes = (0..num_wires)
        .into_par_iter()
        .map(|i| compute_wire_lde(i, &witness, degree, config.rate_bits))
        .collect::<Vec<_>>();
    info!("{:.3}s to compute wire LDEs",
          start_wire_ldes.elapsed().as_secs_f32());

    // TODO: Could try parallelizing the transpose, or not doing it explicitly, instead having
    // merkle_root_bit_rev_order do it implicitly.
    let start_wire_transpose = Instant::now();
    let wire_ldes_t = transpose_poly_values(wire_ldes);
    info!("{:.3}s to transpose wire LDEs",
          start_wire_transpose.elapsed().as_secs_f32());

    // TODO: Could avoid cloning if it's significant?
    let start_wires_root = Instant::now();
    let wires_root = merkle_root_bit_rev_order(wire_ldes_t.clone());
    info!("{:.3}s to Merklize wire LDEs",
          start_wires_root.elapsed().as_secs_f32());

    let mut challenger = Challenger::new();
    challenger.observe_hash(&wires_root);
    let betas = challenger.get_n_challenges(num_checks);
    let gammas = challenger.get_n_challenges(num_checks);

    let start_plonk_z = Instant::now();
    let plonk_z_vecs = compute_zs(&common_data);
    let plonk_z_ldes = PolynomialValues::lde_multiple(plonk_z_vecs, config.rate_bits);
    let plonk_z_ldes_t = transpose_poly_values(plonk_z_ldes);
    info!("{:.3}s to compute Z's and their LDEs",
          start_plonk_z.elapsed().as_secs_f32());

    let start_plonk_z_root = Instant::now();
    let plonk_zs_root = merkle_root_bit_rev_order(plonk_z_ldes_t.clone());
    info!("{:.3}s to Merklize Z's",
          start_plonk_z_root.elapsed().as_secs_f32());

    challenger.observe_hash(&plonk_zs_root);

    let alphas = challenger.get_n_challenges(num_checks);

    // TODO
    let beta = betas[0];
    let gamma = gammas[0];

    let start_vanishing_polys = Instant::now();
    let vanishing_polys = compute_vanishing_polys(
        common_data, prover_data, wire_ldes_t, plonk_z_ldes_t, beta, gamma, &alphas);
    info!("{:.3}s to compute vanishing polys",
          start_vanishing_polys.elapsed().as_secs_f32());

    // Compute the quotient polynomials, aka `t` in the Plonk paper.
    let quotient_polys_start = Instant::now();
    let mut all_quotient_poly_chunk_ldes = Vec::with_capacity(num_checks * quotient_degree);
    for vanishing_poly in vanishing_polys.into_iter() {
        let vanishing_poly_coeff = ifft(vanishing_poly);
        let quotient_poly_coeff = divide_by_z_h(vanishing_poly_coeff, degree);
        // Split t into degree-n chunks.
        let quotient_poly_coeff_chunks = quotient_poly_coeff.chunks(degree);
        let quotient_poly_coeff_ldes = PolynomialCoeffs::lde_multiple(
            quotient_poly_coeff_chunks, config.rate_bits);
        let quotient_poly_chunk_ldes: Vec<PolynomialValues<F>> =
            quotient_poly_coeff_ldes.into_par_iter().map(fft).collect();
        all_quotient_poly_chunk_ldes.extend(quotient_poly_chunk_ldes);
    }
    let quotient_polys_root = merkle_root_bit_rev_order(
        transpose_poly_values(all_quotient_poly_chunk_ldes));
    info!("{:.3}s to compute quotient polys and their LDEs",
          quotient_polys_start.elapsed().as_secs_f32());

    let openings = Vec::new(); // TODO

    info!("{:.3}s for overall witness & proof generation",
          start_proof_gen.elapsed().as_secs_f32());

    Proof {
        wires_root,
        plonk_zs_root,
        quotient_polys_root,
        openings,
    }
}

fn compute_zs<F: Field>(common_data: &CommonCircuitData<F>) -> Vec<PolynomialValues<F>> {
    (0..common_data.config.num_checks)
        .map(|i| compute_z(common_data, i))
        .collect()
}

fn compute_z<F: Field>(common_data: &CommonCircuitData<F>, i: usize) -> PolynomialValues<F> {
    PolynomialValues::zero(common_data.degree()) // TODO
}

// TODO: Parallelize.
fn compute_vanishing_polys<F: Field>(
    common_data: &CommonCircuitData<F>,
    prover_data: &ProverOnlyCircuitData<F>,
    wire_ldes_t: Vec<Vec<F>>,
    plonk_z_lde_t: Vec<Vec<F>>,
    beta: F,
    gamma: F,
    alphas: &[F],
) -> Vec<PolynomialValues<F>> {
    let lde_size = common_data.lde_size();
    let lde_gen = common_data.lde_generator();
    let num_checks = common_data.config.num_checks;

    let points = F::cyclic_subgroup_known_order(lde_gen, lde_size);
    let values: Vec<Vec<F>> = points.into_par_iter().enumerate().map(|(i, x)| {
        let i_next = (i + 1) % lde_size;
        let local_wires = &wire_ldes_t[i];
        let next_wires = &wire_ldes_t[i_next];
        let local_constants = &prover_data.constant_ldes_t[i];
        let next_constants = &prover_data.constant_ldes_t[i_next];
        let local_plonk_zs = &plonk_z_lde_t[i];
        let next_plonk_zs = &plonk_z_lde_t[i_next];
        let s_sigmas = &prover_data.sigma_ldes_t[i];

        debug_assert_eq!(local_wires.len(), common_data.config.num_wires);
        debug_assert_eq!(local_plonk_zs.len(), num_checks);

        let vars = EvaluationVars {
            local_constants,
            next_constants,
            local_wires,
            next_wires,
        };
        compute_vanishing_poly_entry(
            common_data, x, vars, local_plonk_zs, next_plonk_zs, s_sigmas, beta, gamma, alphas)
    }).collect();

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
    beta: F,
    gamma: F,
    alphas: &[F],
) -> Vec<F> {
    let constraint_terms = evaluate_gate_constraints(
        &common_data.gates, common_data.num_gate_constraints, vars);

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
            f_prime *= wire_value + beta * s_id + gamma;
            g_prime *= wire_value + beta * s_sigma + gamma;
        }
        vanishing_v_shift_terms.push(f_prime * z_x - g_prime * z_gz);
    }

    let vanishing_terms = [
        vanishing_z_1_terms,
        vanishing_v_shift_terms,
        constraint_terms,
    ].concat();

    reduce_with_powers_multi(&vanishing_terms, alphas)
}

fn compute_wire_lde<F: Field>(
    input: usize,
    witness: &PartialWitness<F>,
    degree: usize,
    rate_bits: usize,
) -> PolynomialValues<F> {
    let wire_values = (0..degree)
        // Some gates do not use all wires, and we do not require that generators populate unused
        // wires, so some wire values will not be set. We can set these to any value; here we
        // arbitrary pick zero. Ideally we would verify that no constraints operate on these unset
        // wires, but that isn't trivial.
        .map(|gate| witness.try_get_wire(Wire { gate, input }).unwrap_or(F::ZERO))
        .collect();
    PolynomialValues::new(wire_values).lde(rate_bits)
}
