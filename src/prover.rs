use crate::circuit_data::{CommonCircuitData, ProverOnlyCircuitData};
use crate::field::field::Field;
use crate::proof::Proof2;

pub(crate) fn prove2<F: Field>(
    prover_data: &ProverOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
) -> Proof2<F> {
    todo!()
}
