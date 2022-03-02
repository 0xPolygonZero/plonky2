use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
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

    let diff_1 = F::from_canonical_u16(diff as u16);
    let diff_2 = F::from_canonical_u16((diff >> 16) as u16);

    values[COL_SUB_OUTPUT_0] = F::from_canonical_u16(diff as u16);
    values[COL_SUB_OUTPUT_1] = F::from_canonical_u16((diff >> 16) as u16);
    values[COL_SUB_OUTPUT_2] = F::from_canonical_u16(br as u16);
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
    let out_br = local_values[COL_SUB_OUTPUT_2];

    let base = F::from_canonical_u64(1 << 16);
    let base_sqr = F::from_canonical_u64(1 << 32);
    // Note that this can't overflow. Since each output limb has been
    // range checked as 16-bits
    let out = (out_br * base_sqr + out_1) - out_2 * base;

    // NB: Not clear how to compute in_1 - in_2 in one expression for
    // PackedFields: If in_1 < in_2 then the sign extension will
    // happen inside the big field, which is probably wrong. Instead,
    // we *first* subtract in_2 from the expected output, and then
    // subtract in_1; the result then should be zero.
    yield_constr.constraint(is_sub * ((out + in_2) - in_1));

    yield_constr.constraint(is_sub * out_br * (P::ONES - out_br));
}

pub(crate) fn eval_subtraction_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    local_values: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_sub = local_values[IS_SUB];
    let in_1 = local_values[COL_SUB_INPUT_0];
    let in_2 = local_values[COL_SUB_INPUT_1];
    let out_1 = local_values[COL_SUB_OUTPUT_0];
    let out_2 = local_values[COL_SUB_OUTPUT_1];
    let out_br = local_values[COL_SUB_OUTPUT_2];

    let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << 16));
    let base_sqr = builder.constant_extension(F::Extension::from_canonical_u64(1 << 32));

    // Note that this can't overflow. Since each output limb has been
    // range checked as 16-bits.

    let t0 = builder.mul_add_extension(out_br, base_sqr, out_1);
    let t1 = builder.mul_extension(out_2, base);
    // out = (out_br * 2^32 + out_1) - out_2 * 2^16
    let out = builder.sub_extension(t0, t1);

    // diff = (out + in_2) - in_1
    let diff = builder.add_extension(out, in_2);
    let diff = builder.sub_extension(diff, in_1);
    // filtered_diff = is_sub * ((out + in_2) - in_1)
    let filtered_diff = builder.mul_extension(is_sub, diff);

    yield_constr.constraint(builder, filtered_diff);

    let one = builder.one_extension();
    // not_out_br = 1 - out_br
    let not_out_br = builder.sub_extension(one, out_br);
    // br = out_br * (1 - out_br)
    let br = builder.mul_extension(out_br, not_out_br);
    // filtered_br = is_sub * out_br * (1 - out_br)
    let filtered_br = builder.mul_extension(is_sub, br);
    yield_constr.constraint(builder, filtered_br);
}
