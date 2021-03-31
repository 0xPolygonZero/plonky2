use std::time::Instant;

use log::info;
use rayon::prelude::*;

use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::constraint_polynomial::EvaluationVars;
use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::hash::merkle_root_bit_rev_order;
use crate::plonk_common::{reduce_with_powers, eval_l_1};
use crate::polynomial::division::divide_by_z_h;
use crate::polynomial::polynomial::{PolynomialCoeffs, PolynomialValues};
use crate::proof::Proof;
use crate::util::transpose_poly_values;
use crate::wire::Wire;
use crate::witness::PartialWitness;

pub(crate) fn prove<F: Field>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
    inputs: PartialWitness<F>,
) -> Proof<F> {
    let mut witness = inputs;
    let start_witness = Instant::now();
    info!("Running {} generators", prover_data.generators.len());
    generate_partial_witness(&mut witness, &prover_data.generators);
    info!("Witness generation took {}s", start_witness.elapsed().as_secs_f32());

    let start_proof_gen = Instant::now();
    let config = common_data.config;
    let num_wires = config.num_wires;

    let start_wire_ldes = Instant::now();
    let degree = common_data.degree();
    let wire_ldes = (0..num_wires)
        .into_par_iter()
        .map(|i| compute_wire_lde(i, &witness, degree, config.rate_bits))
        .collect::<Vec<_>>();
    info!("Computing wire LDEs took {}s", start_wire_ldes.elapsed().as_secs_f32());

    // TODO: Could try parallelizing the transpose, or not doing it explicitly, instead having
    // merkle_root_bit_rev_order do it implicitly.
    let start_wire_transpose = Instant::now();
    let wire_ldes_t = transpose_poly_values(wire_ldes);
    info!("Transposing wire LDEs took {}s", start_wire_transpose.elapsed().as_secs_f32());

    // TODO: Could avoid cloning if it's significant?
    let start_wires_root = Instant::now();
    let wires_root = merkle_root_bit_rev_order(wire_ldes_t.clone());
    info!("Merklizing wire LDEs took {}s", start_wires_root.elapsed().as_secs_f32());

    let plonk_z_vecs = compute_zs(&common_data);
    let plonk_z_ldes = PolynomialValues::lde_multiple(plonk_z_vecs, config.rate_bits);
    let plonk_z_ldes_t = transpose_poly_values(plonk_z_ldes);
    let plonk_z_root = merkle_root_bit_rev_order(plonk_z_ldes_t.clone());

    let beta = F::ZERO; // TODO
    let gamma = F::ZERO; // TODO
    let alpha = F::ZERO; // TODO

    let start_vanishing_poly = Instant::now();
    let vanishing_poly = compute_vanishing_poly(
        common_data, prover_data, wire_ldes_t, plonk_z_ldes_t, beta, gamma, alpha);
    info!("Computing vanishing poly took {}s", start_vanishing_poly.elapsed().as_secs_f32());

    let quotient_poly_start = Instant::now();
    let vanishing_poly_coeffs = ifft(vanishing_poly);
    let plonk_t = divide_by_z_h(vanishing_poly_coeffs, degree);
    // Split t into degree-n chunks.
    let plonk_t_chunks = plonk_t.chunks(degree);
    info!("Computing quotient poly took {}s", quotient_poly_start.elapsed().as_secs_f32());

    // Need to convert to coeff form and back?
    let plonk_t_ldes = PolynomialCoeffs::lde_multiple(plonk_t_chunks, config.rate_bits);
    let plonk_t_ldes = plonk_t_ldes.into_iter().map(fft).collect();
    let plonk_t_root = merkle_root_bit_rev_order(transpose_poly_values(plonk_t_ldes));

    let openings = Vec::new(); // TODO

    info!("Proof generation took {}s", start_proof_gen.elapsed().as_secs_f32());

    Proof {
        wires_root,
        plonk_z_root,
        plonk_t_root,
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

fn compute_vanishing_poly<F: Field>(
    common_data: &CommonCircuitData<F>,
    prover_data: &ProverOnlyCircuitData<F>,
    wire_ldes_t: Vec<Vec<F>>,
    plonk_z_lde_t: Vec<Vec<F>>,
    beta: F,
    gamma: F,
    alpha: F,
) -> PolynomialValues<F> {
    let lde_size = common_data.lde_size();
    let lde_gen = common_data.lde_generator();

    let mut result = Vec::with_capacity(lde_size);
    let mut point = F::ONE;
    for i in 0..lde_size {
        debug_assert!(point != F::ONE);

        let i_next = (i + 1) % lde_size;
        let local_wires = &wire_ldes_t[i];
        let next_wires = &wire_ldes_t[i_next];
        let local_constants = &prover_data.constant_ldes_t[i];
        let next_constants = &prover_data.constant_ldes_t[i_next];
        let local_plonk_zs = &plonk_z_lde_t[i];
        let next_plonk_zs = &plonk_z_lde_t[i_next];
        let s_sigmas = &prover_data.sigma_ldes_t[i];

        debug_assert_eq!(local_wires.len(), common_data.config.num_wires);
        debug_assert_eq!(local_plonk_zs.len(), common_data.config.num_checks);

        let vars = EvaluationVars {
            local_constants,
            next_constants,
            local_wires,
            next_wires,
        };
        result.push(compute_vanishing_poly_entry(
            common_data, point, vars, local_plonk_zs, next_plonk_zs, s_sigmas, beta, gamma, alpha));

        point *= lde_gen;
    }
    debug_assert_eq!(point, F::ONE);
    PolynomialValues::new(result)
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
    alpha: F,
) -> F {
    let constraint_terms = common_data.evaluate(vars);

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

    reduce_with_powers(vanishing_terms, alpha)
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
