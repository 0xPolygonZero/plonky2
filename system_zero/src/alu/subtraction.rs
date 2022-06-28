use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

pub(crate) fn generate_subtraction<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    let in_1 = values[COL_SUB_INPUT_0].to_canonical_u64() as u32;
    let in_2 = values[COL_SUB_INPUT_1].to_canonical_u64() as u32;

    // in_1 - in_2 == diff - br*2^32
    let (diff, br) = in_1.overflowing_sub(in_2);

    values[COL_SUB_OUTPUT_0] = F::from_canonical_u16(diff as u16);
    values[COL_SUB_OUTPUT_1] = F::from_canonical_u16((diff >> 16) as u16);
    values[COL_SUB_OUTPUT_BORROW] = F::from_bool(br);
}

pub(crate) fn eval_subtraction<F: Field, P: PackedField<Scalar = F>>(
    local_values: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_sub = local_values[IS_SUB];
    let in_1 = local_values[COL_SUB_INPUT_0];
    let in_2 = local_values[COL_SUB_INPUT_1];
    let out_1 = local_values[COL_SUB_OUTPUT_0];
    let out_2 = local_values[COL_SUB_OUTPUT_1];
    let out_br = local_values[COL_SUB_OUTPUT_BORROW];

    let base = F::from_canonical_u64(1 << 16);
    let base_sqr = F::from_canonical_u64(1 << 32);

    let out_br = out_br * base_sqr;
    let lhs = (out_br + in_1) - in_2;
    let rhs = out_1 + out_2 * base;

    yield_constr.constraint(is_sub * (lhs - rhs));

    // We don't need to check that out_br is in {0, 1} because it's
    // checked by boolean::col_bit(0) in the ALU.
}

pub(crate) fn eval_subtraction_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    local_values: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_sub = local_values[IS_SUB];
    let in_1 = local_values[COL_SUB_INPUT_0];
    let in_2 = local_values[COL_SUB_INPUT_1];
    let out_1 = local_values[COL_SUB_OUTPUT_0];
    let out_2 = local_values[COL_SUB_OUTPUT_1];
    let out_br = local_values[COL_SUB_OUTPUT_BORROW];

    let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << 16));
    #[allow(unused)] // TODO
    let base_sqr = builder.constant_extension(F::Extension::from_canonical_u64(1 << 32));

    // lhs = (out_br + in_1) - in_2
    let lhs = builder.add_extension(out_br, in_1);
    let lhs = builder.sub_extension(lhs, in_2);

    // rhs = out_1 + base * out_2
    let rhs = builder.mul_add_extension(out_2, base, out_1);

    // filtered_diff = is_sub * (lhs - rhs)
    let diff = builder.sub_extension(lhs, rhs);
    let filtered_diff = builder.mul_extension(is_sub, diff);

    yield_constr.constraint(builder, filtered_diff);
}
