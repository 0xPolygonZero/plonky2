//! The initial phase of execution, where the kernel code is hashed while being written to memory.
//! The hash is then checked against a precomputed kernel hash.

use std::borrow::Borrow;

use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, NUM_CPU_COLUMNS};
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::keccak_util::keccakf_u32s;
use crate::generation::state::GenerationState;
use crate::keccak_sponge::columns::KECCAK_RATE_U32S;
use crate::memory::segments::Segment;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// We can't process more than `NUM_CHANNELS` bytes per row, since that's all the memory bandwidth
/// we have. We also can't process more than 4 bytes (or the number of bytes in a `u32`), since we
/// want them to fit in a single limb of Keccak input.
const BYTES_PER_ROW: usize = 4;

pub(crate) fn generate_bootstrap_kernel<F: Field>(state: &mut GenerationState<F>) {
    let mut sponge_state = [0u32; 50];
    let mut sponge_input_pos: usize = 0;

    // Iterate through chunks of the code, such that we can write one chunk to memory per row.
    for chunk in &KERNEL
        .padded_code()
        .iter()
        .enumerate()
        .chunks(BYTES_PER_ROW)
    {
        state.current_cpu_row.is_bootstrap_kernel = F::ONE;

        // Write this chunk to memory, while simultaneously packing its bytes into a u32 word.
        let mut packed_bytes: u32 = 0;
        for (channel, (addr, &byte)) in chunk.enumerate() {
            state.set_mem_cpu_current(channel, Segment::Code, addr, byte.into());

            packed_bytes = (packed_bytes << 8) | byte as u32;
        }

        sponge_state[sponge_input_pos] = packed_bytes;
        let keccak = state.current_cpu_row.general.keccak_mut();
        keccak.input_limbs = sponge_state.map(F::from_canonical_u32);
        state.commit_cpu_row();

        sponge_input_pos = (sponge_input_pos + 1) % KECCAK_RATE_U32S;
        // If we just crossed a multiple of KECCAK_RATE_LIMBS, then we've filled the Keccak input
        // buffer, so it's time to absorb.
        if sponge_input_pos == 0 {
            state.current_cpu_row.is_keccak = F::ONE;
            keccakf_u32s(&mut sponge_state);
            let keccak = state.current_cpu_row.general.keccak_mut();
            keccak.output_limbs = sponge_state.map(F::from_canonical_u32);
        }
    }
}

pub(crate) fn eval_bootstrap_kernel<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_CPU_COLUMNS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let local_values: &CpuColumnsView<_> = vars.local_values.borrow();
    let next_values: &CpuColumnsView<_> = vars.next_values.borrow();

    // IS_BOOTSTRAP_KERNEL must have an init value of 1, a final value of 0, and a delta in {0, -1}.
    let local_is_bootstrap = local_values.is_bootstrap_kernel;
    let next_is_bootstrap = next_values.is_bootstrap_kernel;
    yield_constr.constraint_first_row(local_is_bootstrap - P::ONES);
    yield_constr.constraint_last_row(local_is_bootstrap);
    let delta_is_bootstrap = next_is_bootstrap - local_is_bootstrap;
    yield_constr.constraint_transition(delta_is_bootstrap * (delta_is_bootstrap + P::ONES));

    // TODO: Constraints to enforce that, if IS_BOOTSTRAP_KERNEL,
    // - If CLOCK is a multiple of KECCAK_RATE_LIMBS, activate the Keccak CTL, and ensure the output
    //   is copied to the next row (besides the first limb which will immediately be overwritten).
    // - Otherwise, ensure that the Keccak input is copied to the next row (besides the next limb).
    // - The next limb we add to the buffer is also written to memory.

    // If IS_BOOTSTRAP_KERNEL changed (from 1 to 0), check that
    // - the clock is a multiple of KECCAK_RATE_LIMBS (TODO)
    // - the current kernel hash matches a precomputed one
    for (&expected, actual) in KERNEL
        .code_hash
        .iter()
        .zip(local_values.general.keccak().output_limbs)
    {
        let expected = P::from(F::from_canonical_u32(expected));
        let diff = expected - actual;
        yield_constr.constraint_transition(delta_is_bootstrap * diff);
    }
}

pub(crate) fn eval_bootstrap_kernel_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_CPU_COLUMNS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let local_values: &CpuColumnsView<_> = vars.local_values.borrow();
    let next_values: &CpuColumnsView<_> = vars.next_values.borrow();
    let one = builder.one_extension();

    // IS_BOOTSTRAP_KERNEL must have an init value of 1, a final value of 0, and a delta in {0, -1}.
    let local_is_bootstrap = local_values.is_bootstrap_kernel;
    let next_is_bootstrap = next_values.is_bootstrap_kernel;
    let constraint = builder.sub_extension(local_is_bootstrap, one);
    yield_constr.constraint_first_row(builder, constraint);
    yield_constr.constraint_last_row(builder, local_is_bootstrap);
    let delta_is_bootstrap = builder.sub_extension(next_is_bootstrap, local_is_bootstrap);
    let constraint =
        builder.mul_add_extension(delta_is_bootstrap, delta_is_bootstrap, delta_is_bootstrap);
    yield_constr.constraint_transition(builder, constraint);

    // TODO: Constraints to enforce that, if IS_BOOTSTRAP_KERNEL,
    // - If CLOCK is a multiple of KECCAK_RATE_LIMBS, activate the Keccak CTL, and ensure the output
    //   is copied to the next row (besides the first limb which will immediately be overwritten).
    // - Otherwise, ensure that the Keccak input is copied to the next row (besides the next limb).
    // - The next limb we add to the buffer is also written to memory.

    // If IS_BOOTSTRAP_KERNEL changed (from 1 to 0), check that
    // - the clock is a multiple of KECCAK_RATE_LIMBS (TODO)
    // - the current kernel hash matches a precomputed one
    for (&expected, actual) in KERNEL
        .code_hash
        .iter()
        .zip(local_values.general.keccak().output_limbs)
    {
        let expected = builder.constant_extension(F::Extension::from_canonical_u32(expected));
        let diff = builder.sub_extension(expected, actual);
        let constraint = builder.mul_extension(delta_is_bootstrap, diff);
        yield_constr.constraint_transition(builder, constraint);
    }
}
