use crate::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::field::field::Field;

pub(crate) fn verify<F: Field>(
    verifier_data: &VerifierOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F>,
) {
    todo!()
}
