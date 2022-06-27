use plonky2::iop::generator::GeneratedValues;
use plonky2::iop::witness::Witness;
use plonky2_field::types::Field;

use crate::gadgets::arithmetic_u32::U32Target;

pub fn generated_values_set_u32_target<F: Field>(
    buffer: &mut GeneratedValues<F>,
    target: U32Target,
    value: u32,
) {
    buffer.set_target(target.0, F::from_canonical_u32(value))
}

pub fn witness_set_u32_target<W: Witness<F>, F: Field>(
    witness: &mut W,
    target: U32Target,
    value: u32,
) {
    witness.set_target(target.0, F::from_canonical_u32(value))
}
