use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2_util::assume;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::alu::canonical::*;
use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

pub(crate) fn generate_mul_add<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    let factor_0 = values[COL_MUL_ADD_FACTOR_0].to_canonical_u64();
    let factor_1 = values[COL_MUL_ADD_FACTOR_1].to_canonical_u64();
    let addend = values[COL_MUL_ADD_ADDEND].to_canonical_u64();

    // Let the compiler know that each input must fit in 32 bits.
    assume(factor_0 <= u32::MAX as u64);
    assume(factor_1 <= u32::MAX as u64);
    assume(addend <= u32::MAX as u64);

    let output = factor_0 * factor_1 + addend;

    // An advice value used to help verify that the limbs represent a canonical field element.
    values[COL_MUL_ADD_RESULT_CANONICAL_INV] = compute_canonical_inv(output);

    values[COL_MUL_ADD_OUTPUT_0] = F::from_canonical_u16(output as u16);
    values[COL_MUL_ADD_OUTPUT_1] = F::from_canonical_u16((output >> 16) as u16);
    values[COL_MUL_ADD_OUTPUT_2] = F::from_canonical_u16((output >> 32) as u16);
    values[COL_MUL_ADD_OUTPUT_3] = F::from_canonical_u16((output >> 48) as u16);
}

pub(crate) fn eval_mul_add<F: Field, P: PackedField<Scalar = F>>(
    local_values: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_mul = local_values[IS_MUL_ADD];
    let factor_0 = local_values[COL_MUL_ADD_FACTOR_0];
    let factor_1 = local_values[COL_MUL_ADD_FACTOR_1];
    let addend = local_values[COL_MUL_ADD_ADDEND];
    let output_1 = local_values[COL_MUL_ADD_OUTPUT_0];
    let output_2 = local_values[COL_MUL_ADD_OUTPUT_1];
    let output_3 = local_values[COL_MUL_ADD_OUTPUT_2];
    let output_4 = local_values[COL_MUL_ADD_OUTPUT_3];
    let result_canonical_inv = local_values[COL_MUL_ADD_RESULT_CANONICAL_INV];

    let computed_output = factor_0 * factor_1 + addend;
    // TODO: Needs to be filtered by IS_MUL_ADD.
    let output = combine_u16s_check_canonical(
        output_1,
        output_2,
        output_3,
        output_4,
        result_canonical_inv,
        yield_constr,
    );
    yield_constr.constraint(is_mul * (computed_output - output));
}

pub(crate) fn eval_mul_add_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    local_values: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mul = local_values[IS_MUL_ADD];
    let factor_0 = local_values[COL_MUL_ADD_FACTOR_0];
    let factor_1 = local_values[COL_MUL_ADD_FACTOR_1];
    let addend = local_values[COL_MUL_ADD_ADDEND];
    let output_1 = local_values[COL_MUL_ADD_OUTPUT_0];
    let output_2 = local_values[COL_MUL_ADD_OUTPUT_1];
    let output_3 = local_values[COL_MUL_ADD_OUTPUT_2];
    let output_4 = local_values[COL_MUL_ADD_OUTPUT_3];
    let result_canonical_inv = local_values[COL_MUL_ADD_RESULT_CANONICAL_INV];

    let computed_output = builder.mul_add_extension(factor_0, factor_1, addend);
    // TODO: Needs to be filtered by IS_MUL_ADD.
    let output = combine_u16s_check_canonical_circuit(
        builder,
        output_1,
        output_2,
        output_3,
        output_4,
        result_canonical_inv,
        yield_constr,
    );
    let diff = builder.sub_extension(computed_output, output);
    let filtered_diff = builder.mul_extension(is_mul, diff);
    yield_constr.constraint(builder, filtered_diff);
}
