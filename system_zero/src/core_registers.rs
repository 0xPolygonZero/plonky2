use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::column_layout::{
    COL_CLOCK, COL_FRAME_PTR, COL_INSTRUCTION_PTR, COL_RANGE_16, COL_STACK_PTR, NUM_COLUMNS,
};
use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::system_zero::SystemZero;

impl<F: RichField + Extendable<D>, const D: usize> SystemZero<F, D> {
    pub(crate) fn generate_first_row_core_registers(&self, first_values: &mut [F; NUM_COLUMNS]) {
        first_values[COL_CLOCK] = F::ZERO;
        first_values[COL_RANGE_16] = F::ZERO;
        first_values[COL_INSTRUCTION_PTR] = F::ZERO;
        first_values[COL_FRAME_PTR] = F::ZERO;
        first_values[COL_STACK_PTR] = F::ZERO;
    }

    pub(crate) fn generate_next_row_core_registers(
        &self,
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

        next_values[COL_INSTRUCTION_PTR] = todo!();

        next_values[COL_FRAME_PTR] = todo!();

        next_values[COL_STACK_PTR] = todo!();
    }

    #[inline]
    pub(crate) fn eval_core_registers<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // The clock must start with 0, and increment by 1.
        let local_clock = vars.local_values[COL_CLOCK];
        let next_clock = vars.next_values[COL_CLOCK];
        let delta_clock = next_clock - local_clock;
        yield_constr.one_first_row(local_clock);
        yield_constr.one(delta_clock - FE::ONE);

        // The 16-bit table must start with 0, end with 2^16 - 1, and increment by 0 or 1.
        let local_range_16 = vars.local_values[COL_RANGE_16];
        let next_range_16 = vars.next_values[COL_RANGE_16];
        let delta_range_16 = next_range_16 - local_range_16;
        yield_constr.one_first_row(local_range_16);
        yield_constr.one_last_row(local_range_16 - FE::from_canonical_u64((1 << 16) - 1));
        yield_constr.one(delta_range_16 * (delta_range_16 - FE::ONE));

        todo!()
    }

    pub(crate) fn eval_core_registers_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        todo!()
    }
}
