//! Once the CPU execution is over (i.e. reached the `halt` label in the kernel),
//! the CPU trace will be padded with special dummy rows, incurring no memory overhead.

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use super::control_flow::get_halt_pc;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};
use crate::cpu::membus::NUM_GP_CHANNELS;

/// Evaluates constraints for the `halt` flag.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_cpu_cycle: P = COL_MAP.op.iter().map(|&col_i| lv[col_i]).sum();
    let is_cpu_cycle_next: P = COL_MAP.op.iter().map(|&col_i| nv[col_i]).sum();

    let halt_state = P::ONES - is_cpu_cycle;
    let next_halt_state = P::ONES - is_cpu_cycle_next;

    // The halt flag must be boolean.
    yield_constr.constraint(halt_state * (halt_state - P::ONES));
    // Once we reach a padding row, there must be only padding rows.
    yield_constr.constraint_transition(halt_state * (next_halt_state - P::ONES));
    // Check that we're in kernel mode.
    yield_constr.constraint(halt_state * (lv.is_kernel_mode - P::ONES));

    // Padding rows should have their memory channels disabled.
    for i in 0..NUM_GP_CHANNELS {
        let channel = lv.mem_channels[i];
        yield_constr.constraint(halt_state * channel.used);
    }

    // The last row must be a dummy padding row.
    yield_constr.constraint_last_row(halt_state - P::ONES);

    // Also, a padding row's `program_counter` must be at the `halt` label.
    // In particular, it ensures that the first padding row may only be added
    // after we jumped to the `halt` function. Subsequent padding rows may set
    // the `program_counter` to arbitrary values (there's no transition
    // constraints) so we can place this requirement on them too.
    let halt_pc = get_halt_pc::<P::Scalar>();
    yield_constr.constraint(halt_state * (lv.program_counter - halt_pc));
}

/// Circuit version of `eval_packed`.
/// Evaluates constraints for the `halt` flag.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();

    let is_cpu_cycle = builder.add_many_extension(COL_MAP.op.iter().map(|&col_i| lv[col_i]));
    let is_cpu_cycle_next = builder.add_many_extension(COL_MAP.op.iter().map(|&col_i| nv[col_i]));

    let halt_state = builder.sub_extension(one, is_cpu_cycle);
    let next_halt_state = builder.sub_extension(one, is_cpu_cycle_next);

    // The halt flag must be boolean.
    let constr = builder.mul_sub_extension(halt_state, halt_state, halt_state);
    yield_constr.constraint(builder, constr);
    // Once we reach a padding row, there must be only padding rows.
    let constr = builder.mul_sub_extension(halt_state, next_halt_state, halt_state);
    yield_constr.constraint_transition(builder, constr);
    // Check that we're in kernel mode.
    let constr = builder.mul_sub_extension(halt_state, lv.is_kernel_mode, halt_state);
    yield_constr.constraint(builder, constr);

    // Padding rows should have their memory channels disabled.
    for i in 0..NUM_GP_CHANNELS {
        let channel = lv.mem_channels[i];
        let constr = builder.mul_extension(halt_state, channel.used);
        yield_constr.constraint(builder, constr);
    }

    // The last row must be a dummy padding row.
    {
        let one = builder.one_extension();
        let constr = builder.sub_extension(halt_state, one);
        yield_constr.constraint_last_row(builder, constr);
    }

    // Also, a padding row's `program_counter` must be at the `halt` label.
    // In particular, it ensures that the first padding row may only be added
    // after we jumped to the `halt` function. Subsequent padding rows may set
    // the `program_counter` to arbitrary values (there's no transition
    // constraints) so we can place this requirement on them too.
    {
        let halt_pc = get_halt_pc();
        let halt_pc_target = builder.constant_extension(halt_pc);
        let constr = builder.sub_extension(lv.program_counter, halt_pc_target);
        let constr = builder.mul_extension(halt_state, constr);

        yield_constr.constraint(builder, constr);
    }
}
