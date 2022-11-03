use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::{StarkEvaluationTargets, StarkEvaluationVars};

use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::core::*;
use crate::registers::NUM_COLUMNS;

pub(crate) fn generate_first_row_core_registers<F: Field>(first_values: &mut [F; NUM_COLUMNS]) {
    first_values[COL_CLOCK] = F::ZERO;
    first_values[COL_RANGE_16] = F::ZERO;
    first_values[COL_INSTRUCTION_PTR] = F::ZERO;
    first_values[COL_FRAME_PTR] = F::ZERO;
    first_values[COL_STACK_PTR] = F::ZERO;
}

pub(crate) fn generate_next_row_core_registers<F: PrimeField64>(
    local_values: &[F; NUM_COLUMNS],
    next_values: &mut [F; NUM_COLUMNS],
) {
    // We increment the clock by 1.
    next_values[COL_CLOCK] = local_values[COL_CLOCK] + F::ONE;

    // We increment the 16-bit table by 1, unless we've reached the max value of 2^16 - 1, in
    // which case we repeat that value.
    let prev_range_16 = local_values[COL_RANGE_16].to_canonical_u64();
    let next_range_16 = (prev_range_16 + 1).min((1 << 16) - 1);
    next_values[COL_RANGE_16] = F::from_canonical_u64(next_range_16);

    // next_values[COL_INSTRUCTION_PTR] = todo!();

    // next_values[COL_FRAME_PTR] = todo!();

    // next_values[COL_STACK_PTR] = todo!();
}

#[inline]
pub(crate) fn eval_core_registers<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // The clock must start with 0, and increment by 1.
    let local_clock = vars.local_values[COL_CLOCK];
    let next_clock = vars.next_values[COL_CLOCK];
    let delta_clock = next_clock - local_clock;
    yield_constr.constraint_first_row(local_clock);
    yield_constr.constraint_transition(delta_clock - F::ONE);

    // The 16-bit table must start with 0, end with 2^16 - 1, and increment by 0 or 1.
    let local_range_16 = vars.local_values[COL_RANGE_16];
    let next_range_16 = vars.next_values[COL_RANGE_16];
    let delta_range_16 = next_range_16 - local_range_16;
    yield_constr.constraint_first_row(local_range_16);
    yield_constr.constraint_last_row(local_range_16 - F::from_canonical_u64((1 << 16) - 1));
    yield_constr.constraint_transition(delta_range_16 * delta_range_16 - delta_range_16);

    // TODO constraints for stack etc.
}

pub(crate) fn eval_core_registers_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one_ext = builder.one_extension();
    let max_u16 = builder.constant(F::from_canonical_u64((1 << 16) - 1));
    let max_u16_ext = builder.convert_to_ext(max_u16);

    // The clock must start with 0, and increment by 1.
    let local_clock = vars.local_values[COL_CLOCK];
    let next_clock = vars.next_values[COL_CLOCK];
    let delta_clock = builder.sub_extension(next_clock, local_clock);
    yield_constr.constraint_first_row(builder, local_clock);
    let constraint = builder.sub_extension(delta_clock, one_ext);
    yield_constr.constraint_transition(builder, constraint);

    // The 16-bit table must start with 0, end with 2^16 - 1, and increment by 0 or 1.
    let local_range_16 = vars.local_values[COL_RANGE_16];
    let next_range_16 = vars.next_values[COL_RANGE_16];
    let delta_range_16 = builder.sub_extension(next_range_16, local_range_16);
    yield_constr.constraint_first_row(builder, local_range_16);
    let constraint = builder.sub_extension(local_range_16, max_u16_ext);
    yield_constr.constraint_last_row(builder, constraint);
    let constraint = builder.mul_add_extension(delta_range_16, delta_range_16, delta_range_16);
    yield_constr.constraint_transition(builder, constraint);

    // TODO constraints for stack etc.
}
