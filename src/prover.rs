use std::env::var;
use std::time::Instant;

use log::info;
use rayon::prelude::*;

use crate::circuit_data::{CommonCircuitData, ProverCircuitData, ProverOnlyCircuitData};
use crate::constraint_polynomial::EvaluationVars;
use crate::field::fft::{fft, ifft, lde, lde_multiple};
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::hash::{compress, hash_n_to_hash, hash_n_to_m, hash_or_noop, merkle_root_bit_rev_order};
use crate::plonk_common::reduce_with_powers;
use crate::proof::{Hash, Proof};
use crate::util::{log2_ceil, reverse_index_bits, transpose};
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
    let wire_ldes_t = transpose(&wire_ldes);
    info!("Transposing wire LDEs took {}s", start_wire_transpose.elapsed().as_secs_f32());

    // TODO: Could avoid cloning if it's significant?
    let start_wires_root = Instant::now();
    let wires_root = merkle_root_bit_rev_order(wire_ldes_t.clone());
    info!("Merklizing wire LDEs took {}s", start_wires_root.elapsed().as_secs_f32());

    let plonk_z_vecs = compute_zs(&common_data);
    let plonk_z_ldes = lde_multiple(plonk_z_vecs, config.rate_bits);
    let plonk_z_ldes_t = transpose(&plonk_z_ldes);
    let plonk_z_root = merkle_root_bit_rev_order(plonk_z_ldes_t.clone());

    let alpha = F::ZERO; // TODO

    let start_vanishing_poly = Instant::now();
    let vanishing_poly = compute_vanishing_poly(
        common_data, prover_data, wire_ldes_t, plonk_z_ldes_t, alpha);
    info!("Computing vanishing poly took {}s", start_vanishing_poly.elapsed().as_secs_f32());

    let div_z_h_start = Instant::now();
    // TODO
    info!("Division by Z_H took {}s", div_z_h_start.elapsed().as_secs_f32());

    let plonk_t: Vec<F> = todo!(); // vanishing_poly / Z_H
    // Need to convert to coeff form and back?
    let plonk_t_parts = todo!();
    let plonk_t_ldes = lde_multiple(plonk_t_parts, config.rate_bits);
    let plonk_t_root = merkle_root_bit_rev_order(transpose(&plonk_t_ldes));

    let openings = todo!();

    Proof {
        wires_root,
        plonk_z_root,
        plonk_t_root,
        openings,
    }
}

fn compute_zs<F: Field>(common_data: &CommonCircuitData<F>) -> Vec<Vec<F>> {
    (0..common_data.config.num_checks)
        .map(|i| compute_z(common_data, i))
        .collect()
}

fn compute_z<F: Field>(common_data: &CommonCircuitData<F>, i: usize) -> Vec<F> {
    vec![F::ZERO; common_data.degree()] // TODO
}

fn compute_vanishing_poly<F: Field>(
    common_data: &CommonCircuitData<F>,
    prover_data: &ProverOnlyCircuitData<F>,
    wire_ldes_t: Vec<Vec<F>>,
    plonk_z_lde_t: Vec<Vec<F>>,
    alpha: F,
) -> Vec<F> {
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

        debug_assert_eq!(local_wires.len(), common_data.config.num_wires);
        debug_assert_eq!(local_plonk_zs.len(), common_data.config.num_checks);

        let vars = EvaluationVars {
            local_constants,
            next_constants,
            local_wires,
            next_wires,
        };
        result.push(compute_vanishing_poly_entry(
            common_data, vars, local_plonk_zs, next_plonk_zs, alpha));

        point *= lde_gen;
    }
    debug_assert_eq!(point, F::ONE);
    result
}

fn compute_vanishing_poly_entry<F: Field>(
    common_data: &CommonCircuitData<F>,
    vars: EvaluationVars<F>,
    local_plonk_zs: &[F],
    next_plonk_zs: &[F],
    alpha: F,
) -> F {
    let mut constraints = Vec::with_capacity(common_data.total_constraints());
    // TODO: Add Z constraints.
    constraints.extend(common_data.evaluate(vars));
    reduce_with_powers(constraints, alpha)
}

fn compute_wire_lde<F: Field>(
    input: usize,
    witness: &PartialWitness<F>,
    degree: usize,
    rate_bits: usize,
) -> Vec<F> {
    let wire_values = (0..degree)
        // Some gates do not use all wires, and we do not require that generators populate unused
        // wires, so some wire values will not be set. We can set these to any value; here we
        // arbitrary pick zero. Ideally we would verify that no constraints operate on these unset
        // wires, but that isn't trivial.
        .map(|gate| witness.try_get_wire(Wire { gate, input }).unwrap_or(F::ZERO))
        .collect();
    lde(wire_values, rate_bits)
}
