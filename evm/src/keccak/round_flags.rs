use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::keccak::keccak_stark::{NUM_PUBLIC_INPUTS, NUM_ROUNDS};
use crate::keccak::registers::reg_step;
use crate::keccak::registers::NUM_REGISTERS;
use crate::vars::StarkEvaluationTargets;
use crate::vars::StarkEvaluationVars;

pub(crate) fn eval_round_flags<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_REGISTERS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Initially, the first step flag should be 1 while the others should be 0.
    yield_constr.constraint_first_row(vars.local_values[reg_step(0)] - F::ONE);
    for i in 1..NUM_ROUNDS {
        yield_constr.constraint_first_row(vars.local_values[reg_step(i)]);
    }

    // TODO: Transition.
}

pub(crate) fn eval_round_flags_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_REGISTERS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();

    // Initially, the first step flag should be 1 while the others should be 0.
    let step_0_minus_1 = builder.sub_extension(vars.local_values[reg_step(0)], one);
    yield_constr.constraint_first_row(builder, step_0_minus_1);
    for i in 1..NUM_ROUNDS {
        yield_constr.constraint_first_row(builder, vars.local_values[reg_step(i)]);
    }
}
