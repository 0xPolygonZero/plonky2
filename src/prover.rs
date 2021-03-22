use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::fft::{fft, ifft};
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::proof::{Proof2, Hash};
use crate::util::{log2_ceil, transpose};
use crate::wire::Wire;
use crate::witness::PartialWitness;
use crate::hash::{hash_n_to_hash, hash_n_to_m};

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
    let wires_root = merkle_root(wire_ldes);

    let z_ldes = todo!();
    let plonk_z_root = merkle_root(z_ldes);

    let plonk_t_root = todo!();

    let openings = todo!();

    Proof2 {
        wires_root,
        plonk_z_root,
        plonk_t_root,
        openings,
    }
}

/// Given `n` vectors, each of length `l`, constructs a Merkle tree with `l` leaves, where each leaf
/// is a hash obtained by hashing a "leaf set" consisting of `n` elements. If `n <= 4`, this hashing
/// is skipped, as there is no need to compress leaf data.
fn merkle_root<F: Field>(vecs: Vec<Vec<F>>) -> Hash<F> {
    let n = vecs.len();
    let mut vecs_t = transpose(&vecs);
    let l = vecs_t.len();
    if n > 4 {
        vecs_t = vecs_t.into_iter()
            .map(|leaf_set| hash_n_to_hash(leaf_set, false).elements)
            .collect();
    }
    todo!()
}

fn compute_wire_lde<F: Field>(
    input: usize,
    witness: &PartialWitness<F>,
    degree: usize,
    lde_size: usize,
) -> Vec<F> {
    let wire = (0..degree)
        // Some gates do not use all wires, and we do not require that generators populate unused
        // wires, so some wire values will not be set. We can set these to any value; here we
        // arbitrary pick zero. Ideally we would verify that no constraints operate on these unset
        // wires, but that isn't trivial.
        .map(|gate| witness.try_get_wire(Wire { gate, input }).unwrap_or(F::ZERO))
        .collect();
    let mut coeffs = ifft(wire);
    for _ in 0..(lde_size - degree) {
        coeffs.push(F::ZERO);
    }
    fft(coeffs)
}
