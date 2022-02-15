use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::arithmetic::*;
use crate::registers::NUM_COLUMNS;

pub(crate) fn generate_multiplication<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    // TODO
}

pub(crate) fn eval_multiplication<F: Field, P: PackedField<Scalar = F>>(
    local_values: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_mul = local_values[IS_MUL];
    // TODO
}

pub(crate) fn eval_multiplication_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    local_values: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mul = local_values[IS_MUL];
    // TODO
}
