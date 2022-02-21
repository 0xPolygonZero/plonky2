use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::alu::canonical::{
    compute_canonical_inv, eval_u16s_canonical_check, eval_u16s_canonical_check_circuit,
};
use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

pub(crate) fn generate_multiplication<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    let in_1 = values[COL_MUL_ADD_INPUT_1].to_canonical_u64();
    let in_2 = values[COL_MUL_ADD_INPUT_2].to_canonical_u64();
    let in_3 = values[COL_MUL_ADD_INPUT_3].to_canonical_u64();
    let output = in_1 * in_2 + in_3;

    values[COL_MUL_ADD_OUTPUT_1] = F::from_canonical_u16(output as u16);
    values[COL_MUL_ADD_OUTPUT_2] = F::from_canonical_u16((output >> 16) as u16);
    values[COL_MUL_ADD_OUTPUT_3] = F::from_canonical_u16((output >> 32) as u16);
    values[COL_MUL_ADD_OUTPUT_4] = F::from_canonical_u16((output >> 48) as u16);

    values[COL_MUL_ADD_RESULT_CANONICAL_INV] = compute_canonical_inv(output);
}

pub(crate) fn eval_multiplication<F: Field, P: PackedField<Scalar = F>>(
    local_values: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_mul = local_values[IS_MUL];
    let input_1 = local_values[COL_MUL_ADD_INPUT_1];
    let input_2 = local_values[COL_MUL_ADD_INPUT_2];
    let input_3 = local_values[COL_MUL_ADD_INPUT_3];
    let output_1 = local_values[COL_MUL_ADD_OUTPUT_1];
    let output_2 = local_values[COL_MUL_ADD_OUTPUT_2];
    let output_3 = local_values[COL_MUL_ADD_OUTPUT_3];
    let output_4 = local_values[COL_MUL_ADD_OUTPUT_4];
    let result_canonical_inv = local_values[COL_MUL_ADD_RESULT_CANONICAL_INV];

    let computed_output = input_1 * input_2 + input_3;
    let output = eval_u16s_canonical_check(
        output_1,
        output_2,
        output_3,
        output_4,
        result_canonical_inv,
        yield_constr,
    );
    yield_constr.constraint(computed_output - output);
}

pub(crate) fn eval_multiplication_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    local_values: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mul = local_values[IS_MUL];
    let input_1 = local_values[COL_MUL_ADD_INPUT_1];
    let input_2 = local_values[COL_MUL_ADD_INPUT_2];
    let input_3 = local_values[COL_MUL_ADD_INPUT_3];
    let output_1 = local_values[COL_MUL_ADD_OUTPUT_1];
    let output_2 = local_values[COL_MUL_ADD_OUTPUT_2];
    let output_3 = local_values[COL_MUL_ADD_OUTPUT_3];
    let output_4 = local_values[COL_MUL_ADD_OUTPUT_4];
    let result_canonical_inv = local_values[COL_MUL_ADD_RESULT_CANONICAL_INV];

    let computed_output = builder.mul_add_extension(input_1, input_2, input_3);
    let output = eval_u16s_canonical_check_circuit(
        builder,
        output_1,
        output_2,
        output_3,
        output_4,
        result_canonical_inv,
        yield_constr,
    );
    let diff = builder.sub_extension(computed_output, output);
    yield_constr.constraint(builder, diff);
}
