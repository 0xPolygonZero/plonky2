use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::plonk_common::reduce_with_powers_ext_circuit;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

pub(crate) fn generate_addition<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    let in_1 = values[COL_ADD_INPUT_0].to_canonical_u64();
    let in_2 = values[COL_ADD_INPUT_1].to_canonical_u64();
    let in_3 = values[COL_ADD_INPUT_2].to_canonical_u64();
    let output = in_1 + in_2 + in_3;

    values[COL_ADD_OUTPUT_0] = F::from_canonical_u16(output as u16);
    values[COL_ADD_OUTPUT_1] = F::from_canonical_u16((output >> 16) as u16);
    values[COL_ADD_OUTPUT_2] = F::from_canonical_u16((output >> 32) as u16);
}

pub(crate) fn eval_addition<F: Field, P: PackedField<Scalar = F>>(
    local_values: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_add = local_values[IS_ADD];
    let in_1 = local_values[COL_ADD_INPUT_0];
    let in_2 = local_values[COL_ADD_INPUT_1];
    let in_3 = local_values[COL_ADD_INPUT_2];
    let out_1 = local_values[COL_ADD_OUTPUT_0];
    let out_2 = local_values[COL_ADD_OUTPUT_1];
    let out_3 = local_values[COL_ADD_OUTPUT_2];

    let weight_2 = F::from_canonical_u64(1 << 16);
    let weight_3 = F::from_canonical_u64(1 << 32);
    // Note that this can't overflow. Since each output limb has been range checked as 16-bits,
    // this sum can be around 48 bits at most.
    let out = out_1 + out_2 * weight_2 + out_3 * weight_3;

    let computed_out = in_1 + in_2 + in_3;

    yield_constr.constraint(is_add * (out - computed_out));
}

pub(crate) fn eval_addition_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    local_values: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = local_values[IS_ADD];
    let in_1 = local_values[COL_ADD_INPUT_0];
    let in_2 = local_values[COL_ADD_INPUT_1];
    let in_3 = local_values[COL_ADD_INPUT_2];
    let out_1 = local_values[COL_ADD_OUTPUT_0];
    let out_2 = local_values[COL_ADD_OUTPUT_1];
    let out_3 = local_values[COL_ADD_OUTPUT_2];

    let limb_base = builder.constant(F::from_canonical_u64(1 << 16));
    // Note that this can't overflow. Since each output limb has been range checked as 16-bits,
    // this sum can be around 48 bits at most.
    let out = reduce_with_powers_ext_circuit(builder, &[out_1, out_2, out_3], limb_base);

    let computed_out = builder.add_many_extension([in_1, in_2, in_3]);

    let diff = builder.sub_extension(out, computed_out);
    let filtered_diff = builder.mul_extension(is_add, diff);
    yield_constr.constraint(builder, filtered_diff);
}
