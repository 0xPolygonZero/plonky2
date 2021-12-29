use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::hash::poseidon;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::column_layout::{col_permutation_input, col_permutation_output, NUM_COLUMNS};
use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::system_zero::SystemZero;

impl<F: RichField + Extendable<D>, const D: usize> SystemZero<F, D> {
    pub(crate) fn generate_permutation_unit(&self, values: &mut [F; NUM_COLUMNS]) {
        // Load inputs.
        let mut state = [F::ZERO; SPONGE_WIDTH];
        for i in 0..SPONGE_WIDTH {
            state[i] = values[col_permutation_input(i)];
        }

        // TODO: First full rounds.
        // TODO: Partial rounds.
        // TODO: Second full rounds.

        // Write outputs.
        for i in 0..SPONGE_WIDTH {
            values[col_permutation_output(i)] = state[i];
        }
    }

    #[inline]
    pub(crate) fn eval_permutation_unit<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let local_values = &vars.local_values;

        // Load inputs.
        let mut state = [P::ZEROS; SPONGE_WIDTH];
        for i in 0..SPONGE_WIDTH {
            state[i] = local_values[col_permutation_input(i)];
        }

        // TODO: First full rounds.
        // TODO: Partial rounds.
        // TODO: Second full rounds.

        // Assert that the computed output matches the outputs in the witness.
        for i in 0..SPONGE_WIDTH {
            let out = local_values[col_permutation_output(i)];
            yield_constr.one(state[i] - out);
        }
    }

    pub(crate) fn eval_permutation_unit_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let zero = builder.zero_extension();
        let local_values = &vars.local_values;

        // Load inputs.
        let mut state = [zero; SPONGE_WIDTH];
        for i in 0..SPONGE_WIDTH {
            state[i] = local_values[col_permutation_input(i)];
        }

        // TODO: First full rounds.
        // TODO: Partial rounds.
        // TODO: Second full rounds.

        // Assert that the computed output matches the outputs in the witness.
        for i in 0..SPONGE_WIDTH {
            let out = local_values[col_permutation_output(i)];
            let diff = builder.sub_extension(state[i], out);
            yield_constr.one(builder, diff);
        }
    }
}
