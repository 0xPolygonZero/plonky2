use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

/// General-purpose memory channels; they can read and write to all contexts/segments/addresses.
pub(crate) const NUM_GP_CHANNELS: usize = 3;

/// Indices for code and general purpose memory channels.
pub mod channel_indices {
    use std::ops::Range;

    pub(crate) const CODE: usize = 0;
    pub(crate) const GP: Range<usize> = CODE + 1..(CODE + 1) + super::NUM_GP_CHANNELS;
}

/// Total memory channels used by the CPU table. This includes all the `GP_MEM_CHANNELS` as well as
/// all special-purpose memory channels.
///
/// Currently, there is one special-purpose memory channel, which reads the opcode from memory. Its
/// limitations are:
///  - it is enabled by `is_cpu_cycle`,
///  - it always reads and cannot write,
///  - the context is derived from the current context and the `is_kernel_mode` flag,
///  - the segment is hard-wired to the code segment,
///  - the address is `program_counter`,
///  - the value must fit in one byte (in the least-significant position) and its eight bits are
///    found in `opcode_bits`.
///
/// There is also a partial channel, which shares its values with another general purpose channel.
///
/// These limitations save us numerous columns in the CPU table.
pub(crate) const NUM_CHANNELS: usize = channel_indices::GP.end + 1;

/// Evaluates constraints regarding the membus.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Validate `lv.code_context`.
    // It should be 0 if in kernel mode and `lv.context` if in user mode.
    yield_constr.constraint(lv.code_context - (P::ONES - lv.is_kernel_mode) * lv.context);

    // Validate `channel.used`. It should be binary.
    for channel in lv.mem_channels {
        yield_constr.constraint(channel.used * (channel.used - P::ONES));
    }

    // Validate `partial_channel.used`. It should be binary.
    yield_constr.constraint(lv.partial_channel.used * (lv.partial_channel.used - P::ONES));
}

/// Circuit version of `eval_packed`.
/// Evaluates constraints regarding the membus.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // Validate `lv.code_context`.
    // It should be 0 if in kernel mode and `lv.context` if in user mode.
    let diff = builder.sub_extension(lv.context, lv.code_context);
    let constr = builder.mul_sub_extension(lv.is_kernel_mode, lv.context, diff);
    yield_constr.constraint(builder, constr);

    // Validate `channel.used`. It should be binary.
    for channel in lv.mem_channels {
        let constr = builder.mul_sub_extension(channel.used, channel.used, channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Validate `partial_channel.used`. It should be binary.
    {
        let constr = builder.mul_sub_extension(
            lv.partial_channel.used,
            lv.partial_channel.used,
            lv.partial_channel.used,
        );
        yield_constr.constraint(builder, constr);
    }
}
