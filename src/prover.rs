use std::time::Instant;

use log::info;

use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::fft::{fft, ifft, lde};
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::hash::{compress, hash_n_to_hash, hash_n_to_m, hash_or_noop, merkle_root_bit_rev_order};
use crate::proof::{Hash, Proof2};
use crate::util::{log2_ceil, reverse_index_bits};
use crate::wire::Wire;
use crate::witness::PartialWitness;
use rayon::prelude::*;

pub(crate) fn prove<F: Field>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
    inputs: PartialWitness<F>,
) -> Proof2<F> {
    let mut witness = inputs;
    let start_witness = Instant::now();
    info!("Running {} generators", prover_data.generators.len());
    generate_partial_witness(&mut witness, &prover_data.generators);
    info!("Witness generation took {}s", start_witness.elapsed().as_secs_f32());

    let config = common_data.config;
    let num_wires = config.num_wires;

    let start_wire_ldes = Instant::now();
    // TODO: Simplify using lde_multiple.
    // TODO: Parallelize.
    let wire_ldes = (0..num_wires)
        .map(|i| compute_wire_lde(i, &witness, common_data.degree, config.rate_bits))
        .collect::<Vec<_>>();
    info!("Computing wire LDEs took {}s", start_wire_ldes.elapsed().as_secs_f32());

    let start_wires_root = Instant::now();
    let wires_root = merkle_root_bit_rev_order(wire_ldes);
    info!("Merklizing wire LDEs took {}s", start_wires_root.elapsed().as_secs_f32());

    let plonk_z_vecs = todo!();
    let plonk_z_ldes = todo!();
    let plonk_z_root = merkle_root_bit_rev_order(plonk_z_ldes);

    let plonk_t_vecs = todo!();
    let plonk_t_ldes = todo!();
    let plonk_t_root = merkle_root_bit_rev_order(plonk_t_ldes);

    let openings = todo!();

    Proof2 {
        wires_root,
        plonk_z_root,
        plonk_t_root,
        openings,
    }
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
