use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::proof::{Proof2, Hash};
use crate::util::log2_ceil;
use crate::wire::Wire;
use crate::witness::PartialWitness;

pub(crate) fn prove<F: Field>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
    inputs: PartialWitness<F>,
) -> Proof2<F> {
    let mut witness = inputs;
    generate_partial_witness(&mut witness, &prover_data.generators);

    let config = common_data.config;
    let constraint_degree = 1 << log2_ceil(common_data.constraint_degree(config));
    let lde_size = constraint_degree * common_data.degree;

    let num_wires = config.num_wires;
    let wire_ldes = (0..num_wires)
        .map(|i| compute_wire_lde(i, &witness, common_data.degree, lde_size))
        .collect::<Vec<_>>();
    let wires_root = merkle_root_batch(wire_ldes);

    let z_ldes = todo!();
    let plonk_z_root = merkle_root_batch(z_ldes);

    let plonk_t_root = todo!();

    let openings = todo!();

    Proof2 {
        wires_root,
        plonk_z_root,
        plonk_t_root,
        openings,
    }
}

fn merkle_root<F: Field>(vec: Vec<F>) -> Hash<F> {
    todo!()
}

fn merkle_root_batch<F: Field>(vecs: Vec<Vec<F>>) -> Hash<F> {
    todo!()
}

fn compute_wire_lde<F: Field>(
    input: usize,
    witness: &PartialWitness<F>,
    degree: usize,
    lde_size: usize,
) -> Vec<F> {
    let wire = (0..degree)
        .map(|gate| witness.get_wire(Wire { gate, input }))
        .collect();
    let mut coeffs = ifft(wire);
    for _ in 0..(lde_size - degree) {
        coeffs.push(F::ZERO);
    }
    fft(coeffs)
}
