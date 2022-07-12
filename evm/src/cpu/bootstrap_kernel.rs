//! The initial phase of execution, where the kernel code is hashed while being written to memory.
//! The hash is then checked against a precomputed kernel hash.

use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{COL_MAP, NUM_CPU_COLUMNS};
use crate::cpu::public_inputs::NUM_PUBLIC_INPUTS;
use crate::generation::state::GenerationState;
use crate::memory;
use crate::memory::segments;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

pub(crate) fn generate_bootstrap_kernel<F: Field>(state: &mut GenerationState<F>) {
    for chunk in &state.kernel.code.clone().into_iter().enumerate().chunks(4) {
        for (addr, byte) in chunk {
            let mut value = [F::ZERO; memory::VALUE_LIMBS];
            value[0] = F::from_canonical_u8(byte);

            let channel = addr % memory::NUM_CHANNELS;
            state.set_mem_current(channel, segments::CODE, addr, value);

            // TODO: Set other registers.

            state.commit_cpu_row();
        }
    }
}

pub(crate) fn eval_bootstrap_kernel<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_CPU_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // IS_BOOTSTRAP_KERNEL must have an init value of 1, a final value of 0, and a delta in {0, -1}.
    let local_is_bootstrap = vars.local_values[COL_MAP.is_bootstrap_kernel];
    let next_is_bootstrap = vars.next_values[COL_MAP.is_bootstrap_kernel];
    yield_constr.constraint_first_row(local_is_bootstrap - P::ONES);
    yield_constr.constraint_last_row(local_is_bootstrap);
    let delta_is_bootstrap = next_is_bootstrap - local_is_bootstrap;
    yield_constr.constraint_transition(delta_is_bootstrap * (delta_is_bootstrap + P::ONES));

    // If IS_BOOTSTRAP_KERNEL changed (from 1 to 0), check that the current kernel hash matches a
    // precomputed one.
    let hash_diff = F::ZERO; // TODO
    yield_constr.constraint_transition(delta_is_bootstrap * hash_diff)
}

pub(crate) fn eval_bootstrap_kernel_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_CPU_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();

    // IS_BOOTSTRAP_KERNEL must have an init value of 1, a final value of 0, and a delta in {0, -1}.
    let local_is_bootstrap = vars.local_values[COL_MAP.is_bootstrap_kernel];
    let next_is_bootstrap = vars.next_values[COL_MAP.is_bootstrap_kernel];
    let constraint = builder.sub_extension(local_is_bootstrap, one);
    yield_constr.constraint_first_row(builder, constraint);
    yield_constr.constraint_last_row(builder, local_is_bootstrap);
    let delta_is_bootstrap = builder.sub_extension(next_is_bootstrap, local_is_bootstrap);
    let constraint =
        builder.mul_add_extension(delta_is_bootstrap, delta_is_bootstrap, delta_is_bootstrap);
    yield_constr.constraint_transition(builder, constraint);

    // If IS_BOOTSTRAP_KERNEL changed (from 1 to 0), check that the current kernel hash matches a
    // precomputed one.
    let hash_diff = builder.zero_extension(); // TODO
    let constraint = builder.mul_extension(delta_is_bootstrap, hash_diff);
    yield_constr.constraint_transition(builder, constraint)
}
