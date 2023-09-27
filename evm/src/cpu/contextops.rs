use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::columns::ops::OpsColumnsView;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

// If true, the instruction will keep the current context for the next row.
// If false, next row's context is handled manually.
const KEEPS_CONTEXT: OpsColumnsView<bool> = OpsColumnsView {
    binary_op: true,
    ternary_op: true,
    fp254_op: true,
    eq_iszero: true,
    logic_op: true,
    not: true,
    shift: true,
    keccak_general: true,
    prover_input: true,
    pop: true,
    jumps: true,
    pc: true,
    jumpdest: true,
    push0: true,
    push: true,
    dup: true,
    swap: true,
    context_op: false,
    mstore_32bytes: true,
    mload_32bytes: true,
    exit_kernel: true,
    m_op_general: true,
    syscall: true,
    exception: true,
};

fn eval_packed_keep<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    for (op, keeps_context) in izip!(lv.op.into_iter(), KEEPS_CONTEXT.into_iter()) {
        if keeps_context {
            yield_constr.constraint_transition(op * (nv.context - lv.context));
        }
    }
}

fn eval_ext_circuit_keep<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    for (op, keeps_context) in izip!(lv.op.into_iter(), KEEPS_CONTEXT.into_iter()) {
        if keeps_context {
            let diff = builder.sub_extension(nv.context, lv.context);
            let constr = builder.mul_extension(op, diff);
            yield_constr.constraint_transition(builder, constr);
        }
    }
}

fn eval_packed_get<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // If the opcode is GET_CONTEXT, then lv.opcode_bits[0] = 0
    let filter = lv.op.context_op * (P::ONES - lv.opcode_bits[0]);
    let push_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];
    yield_constr.constraint(filter * (push_channel.value[0] - lv.context));
    for &limb in &push_channel.value[1..] {
        yield_constr.constraint(filter * limb);
    }

    // Stack constraints
    let channel = lv.mem_channels[NUM_GP_CHANNELS - 1];
    yield_constr.constraint(filter * (channel.used - P::ONES));
    yield_constr.constraint(filter * channel.is_read);

    yield_constr.constraint(filter * (channel.addr_context - lv.context));
    yield_constr.constraint(
        filter * (channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
    );
    let addr_virtual = lv.stack_len;
    yield_constr.constraint(filter * (channel.addr_virtual - addr_virtual));

    // Keep context
    yield_constr.constraint_transition(filter * (nv.context - lv.context));

    // Unused channels
    for i in 0..NUM_GP_CHANNELS - 1 {
        let channel = lv.mem_channels[i];
        yield_constr.constraint(filter * channel.used);
    }
}

fn eval_ext_circuit_get<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let mut filter = lv.op.context_op;
    let one = builder.one_extension();
    let minus = builder.sub_extension(one, lv.opcode_bits[0]);
    filter = builder.mul_extension(filter, minus);

    let push_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];
    {
        let diff = builder.sub_extension(push_channel.value[0], lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &push_channel.value[1..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }

    // Stack constraints
    let channel = lv.mem_channels[NUM_GP_CHANNELS - 1];

    {
        let constr = builder.mul_sub_extension(filter, channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_extension(filter, channel.is_read);
        yield_constr.constraint(builder, constr);
    }

    {
        let diff = builder.sub_extension(channel.addr_context, lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.arithmetic_extension(
            F::ONE,
            -F::from_canonical_u64(Segment::Stack as u64),
            filter,
            channel.addr_segment,
            filter,
        );
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(channel.addr_virtual, lv.stack_len);
        let constr = builder.arithmetic_extension(F::ONE, F::ZERO, filter, diff, filter);
        yield_constr.constraint(builder, constr);
    }

    // Keep context
    {
        let diff = builder.sub_extension(nv.context, lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }

    // Unused channels
    for i in 0..NUM_GP_CHANNELS - 1 {
        let channel = lv.mem_channels[i];
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }
}

fn eval_packed_set<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.context_op * lv.opcode_bits[0];
    let pop_channel = lv.mem_channels[0];
    let write_old_sp_channel = lv.mem_channels[1];
    let read_new_sp_channel = lv.mem_channels[2];
    let stack_segment = P::Scalar::from_canonical_u64(Segment::Stack as u64);
    let ctx_metadata_segment = P::Scalar::from_canonical_u64(Segment::ContextMetadata as u64);
    let stack_size_field = P::Scalar::from_canonical_u64(ContextMetadata::StackSize as u64);
    let local_sp_dec = lv.stack_len - P::ONES;

    // The next row's context is read from memory channel 0.
    yield_constr.constraint(filter * (pop_channel.value[0] - nv.context));
    yield_constr.constraint(filter * (pop_channel.used - P::ONES));
    yield_constr.constraint(filter * (pop_channel.is_read - P::ONES));
    yield_constr.constraint(filter * (pop_channel.addr_context - lv.context));
    yield_constr.constraint(filter * (pop_channel.addr_segment - stack_segment));
    yield_constr.constraint(filter * (pop_channel.addr_virtual - local_sp_dec));

    // The old SP is decremented (since the new context was popped) and written to memory.
    yield_constr.constraint(filter * (write_old_sp_channel.value[0] - local_sp_dec));
    for limb in &write_old_sp_channel.value[1..] {
        yield_constr.constraint(filter * *limb);
    }
    yield_constr.constraint(filter * (write_old_sp_channel.used - P::ONES));
    yield_constr.constraint(filter * write_old_sp_channel.is_read);
    yield_constr.constraint(filter * (write_old_sp_channel.addr_context - lv.context));
    yield_constr.constraint(filter * (write_old_sp_channel.addr_segment - ctx_metadata_segment));
    yield_constr.constraint(filter * (write_old_sp_channel.addr_virtual - stack_size_field));

    // The new SP is loaded from memory.
    yield_constr.constraint(filter * (read_new_sp_channel.value[0] - nv.stack_len));
    yield_constr.constraint(filter * (read_new_sp_channel.used - P::ONES));
    yield_constr.constraint(filter * (read_new_sp_channel.is_read - P::ONES));
    yield_constr.constraint(filter * (read_new_sp_channel.addr_context - nv.context));
    yield_constr.constraint(filter * (read_new_sp_channel.addr_segment - ctx_metadata_segment));
    yield_constr.constraint(filter * (read_new_sp_channel.addr_virtual - stack_size_field));

    // Disable unused memory channels
    for &channel in &lv.mem_channels[3..] {
        yield_constr.constraint(filter * channel.used);
    }
}

fn eval_ext_circuit_set<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let mut filter = lv.op.context_op;
    filter = builder.mul_extension(filter, lv.opcode_bits[0]);
    let pop_channel = lv.mem_channels[0];
    let write_old_sp_channel = lv.mem_channels[1];
    let read_new_sp_channel = lv.mem_channels[2];
    let stack_segment =
        builder.constant_extension(F::Extension::from_canonical_u32(Segment::Stack as u32));
    let ctx_metadata_segment = builder.constant_extension(F::Extension::from_canonical_u32(
        Segment::ContextMetadata as u32,
    ));
    let stack_size_field = builder.constant_extension(F::Extension::from_canonical_u32(
        ContextMetadata::StackSize as u32,
    ));
    let one = builder.one_extension();
    let local_sp_dec = builder.sub_extension(lv.stack_len, one);

    // The next row's context is read from memory channel 0.
    {
        let diff = builder.sub_extension(pop_channel.value[0], nv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, pop_channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, pop_channel.is_read, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(pop_channel.addr_context, lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(pop_channel.addr_segment, stack_segment);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(pop_channel.addr_virtual, local_sp_dec);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // The old SP is decremented (since the new context was popped) and written to memory.
    {
        let diff = builder.sub_extension(write_old_sp_channel.value[0], local_sp_dec);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for limb in &write_old_sp_channel.value[1..] {
        let constr = builder.mul_extension(filter, *limb);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, write_old_sp_channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_extension(filter, write_old_sp_channel.is_read);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(write_old_sp_channel.addr_context, lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(write_old_sp_channel.addr_segment, ctx_metadata_segment);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(write_old_sp_channel.addr_virtual, stack_size_field);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // The new SP is loaded from memory.
    {
        let diff = builder.sub_extension(read_new_sp_channel.value[0], nv.stack_len);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, read_new_sp_channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, read_new_sp_channel.is_read, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(read_new_sp_channel.addr_context, nv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(read_new_sp_channel.addr_segment, ctx_metadata_segment);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(read_new_sp_channel.addr_virtual, stack_size_field);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // Disable unused memory channels
    for &channel in &lv.mem_channels[3..] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_keep(lv, nv, yield_constr);
    eval_packed_get(lv, nv, yield_constr);
    eval_packed_set(lv, nv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_keep(builder, lv, nv, yield_constr);
    eval_ext_circuit_get(builder, lv, nv, yield_constr);
    eval_ext_circuit_set(builder, lv, nv, yield_constr);
}
