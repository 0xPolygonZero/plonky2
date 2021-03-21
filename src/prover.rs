use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::field::Field;
use crate::generator::generate_partial_witness;
use crate::proof::Proof2;
use crate::witness::PartialWitness;

pub(crate) fn prove<F: Field>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
    inputs: PartialWitness<F>,
) -> Proof2<F> {
    let mut witness = inputs;
    generate_partial_witness(&mut witness, &prover_data.generators);

    Proof2 {
        wires_root: todo!(),
        plonk_z_root: todo!(),
        plonk_t_root: todo!(),
        openings: todo!(),
    }
}
