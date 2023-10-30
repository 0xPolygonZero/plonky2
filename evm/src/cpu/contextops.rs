use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::membus::NUM_GP_CHANNELS;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::memory::segments::Segment;

/// Evaluates constraints for GET_CONTEXT.
fn eval_packed_get<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // If the opcode is GET_CONTEXT, then lv.opcode_bits[0] = 0.
    let filter = lv.op.context_op * (P::ONES - lv.opcode_bits[0]);
    let new_stack_top = nv.mem_channels[0].value;
    yield_constr.constraint(filter * (new_stack_top[0] - lv.context));
    for &limb in &new_stack_top[1..] {
        yield_constr.constraint(filter * limb);
    }

    // Constrain new stack length.
    yield_constr.constraint(filter * (nv.stack_len - (lv.stack_len + P::ONES)));

    // Unused channels.
    for i in 1..NUM_GP_CHANNELS {
        if i != 3 {
            let channel = lv.mem_channels[i];
            yield_constr.constraint(filter * channel.used);
        }
    }
    yield_constr.constraint(filter * nv.mem_channels[0].used);
}

/// Circuit version of `eval_packed_get`.
/// Evalutes constraints for GET_CONTEXT.
fn eval_ext_circuit_get<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // If the opcode is GET_CONTEXT, then lv.opcode_bits[0] = 0.
    let prod = builder.mul_extension(lv.op.context_op, lv.opcode_bits[0]);
    let filter = builder.sub_extension(lv.op.context_op, prod);
    let new_stack_top = nv.mem_channels[0].value;
    {
        let diff = builder.sub_extension(new_stack_top[0], lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &new_stack_top[1..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }

    // Constrain new stack length.
    {
        let new_len = builder.add_const_extension(lv.stack_len, F::ONE);
        let diff = builder.sub_extension(nv.stack_len, new_len);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // Unused channels.
    for i in 1..NUM_GP_CHANNELS {
        if i != 3 {
            let channel = lv.mem_channels[i];
            let constr = builder.mul_extension(filter, channel.used);
            yield_constr.constraint(builder, constr);
        }
    }
    {
        let constr = builder.mul_extension(filter, nv.mem_channels[0].used);
        yield_constr.constraint(builder, constr);
    }
}

/// Evaluates constraints for `SET_CONTEXT`.
fn eval_packed_set<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.context_op * lv.opcode_bits[0];
    let stack_top = lv.mem_channels[0].value;
    let write_old_sp_channel = lv.mem_channels[1];
    let read_new_sp_channel = lv.mem_channels[2];
    let ctx_metadata_segment = P::Scalar::from_canonical_u64(Segment::ContextMetadata as u64);
    let stack_size_field = P::Scalar::from_canonical_u64(ContextMetadata::StackSize as u64);
    let local_sp_dec = lv.stack_len - P::ONES;

    // The next row's context is read from stack_top.
    yield_constr.constraint(filter * (stack_top[0] - nv.context));

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

    // Constrain stack_inv_aux_2.
    let new_top_channel = nv.mem_channels[0];
    yield_constr.constraint(
        lv.op.context_op
            * (lv.general.stack().stack_inv_aux * lv.opcode_bits[0]
                - lv.general.stack().stack_inv_aux_2),
    );
    // The new top is loaded in memory channel 3, if the stack isn't empty (see eval_packed).
    yield_constr.constraint(
        lv.op.context_op
            * lv.general.stack().stack_inv_aux_2
            * (lv.mem_channels[3].value[0] - new_top_channel.value[0]),
    );
    for &limb in &new_top_channel.value[1..] {
        yield_constr.constraint(lv.op.context_op * lv.general.stack().stack_inv_aux_2 * limb);
    }

    // Unused channels.
    for i in 4..NUM_GP_CHANNELS {
        let channel = lv.mem_channels[i];
        yield_constr.constraint(filter * channel.used);
    }
    yield_constr.constraint(filter * new_top_channel.used);
}

/// Circuit version of `eval_packed_set`.
/// Evaluates constraints for SET_CONTEXT.
fn eval_ext_circuit_set<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = builder.mul_extension(lv.op.context_op, lv.opcode_bits[0]);
    let stack_top = lv.mem_channels[0].value;
    let write_old_sp_channel = lv.mem_channels[1];
    let read_new_sp_channel = lv.mem_channels[2];
    let ctx_metadata_segment = builder.constant_extension(F::Extension::from_canonical_u32(
        Segment::ContextMetadata as u32,
    ));
    let stack_size_field = builder.constant_extension(F::Extension::from_canonical_u32(
        ContextMetadata::StackSize as u32,
    ));
    let one = builder.one_extension();
    let local_sp_dec = builder.sub_extension(lv.stack_len, one);

    // The next row's context is read from stack_top.
    {
        let diff = builder.sub_extension(stack_top[0], nv.context);
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

    // Constrain stack_inv_aux_2.
    let new_top_channel = nv.mem_channels[0];
    {
        let diff = builder.mul_sub_extension(
            lv.general.stack().stack_inv_aux,
            lv.opcode_bits[0],
            lv.general.stack().stack_inv_aux_2,
        );
        let constr = builder.mul_extension(lv.op.context_op, diff);
        yield_constr.constraint(builder, constr);
    }
    // The new top is loaded in memory channel 3, if the stack isn't empty (see eval_packed).
    {
        let diff = builder.sub_extension(lv.mem_channels[3].value[0], new_top_channel.value[0]);
        let prod = builder.mul_extension(lv.general.stack().stack_inv_aux_2, diff);
        let constr = builder.mul_extension(lv.op.context_op, prod);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &new_top_channel.value[1..] {
        let prod = builder.mul_extension(lv.general.stack().stack_inv_aux_2, limb);
        let constr = builder.mul_extension(lv.op.context_op, prod);
        yield_constr.constraint(builder, constr);
    }

    // Unused channels.
    for i in 4..NUM_GP_CHANNELS {
        let channel = lv.mem_channels[i];
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_extension(filter, new_top_channel.used);
        yield_constr.constraint(builder, constr);
    }
}

/// Evaluates the constraints for the GET and SET opcodes.
pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_get(lv, nv, yield_constr);
    eval_packed_set(lv, nv, yield_constr);

    // Stack constraints.
    // Both operations use memory channel 3. The operations are similar enough that
    // we can constrain both at the same time.
    let filter = lv.op.context_op;
    let channel = lv.mem_channels[3];
    // For get_context, we check if lv.stack_len is 0. For set_context, we check if nv.stack_len is 0.
    // However, for get_context, we can deduce lv.stack_len from nv.stack_len since the operation only pushes.
    let stack_len = nv.stack_len - (P::ONES - lv.opcode_bits[0]);
    // Constrain stack_inv_aux. It's 0 if the relevant stack is empty, 1 otherwise.
    yield_constr.constraint(
        filter * (stack_len * lv.general.stack().stack_inv - lv.general.stack().stack_inv_aux),
    );
    // Enable or disable the channel.
    yield_constr.constraint(filter * (lv.general.stack().stack_inv_aux - channel.used));
    let new_filter = filter * lv.general.stack().stack_inv_aux;
    // It's a write for get_context, a read for set_context.
    yield_constr.constraint(new_filter * (channel.is_read - lv.opcode_bits[0]));
    // In both cases, next row's context works.
    yield_constr.constraint(new_filter * (channel.addr_context - nv.context));
    // Same segment for both.
    yield_constr.constraint(
        new_filter * (channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
    );
    // The address is one less than stack_len.
    let addr_virtual = stack_len - P::ONES;
    yield_constr.constraint(new_filter * (channel.addr_virtual - addr_virtual));
}

/// Circuit version of èval_packed`.
/// Evaluates the constraints for the GET and SET opcodes.
pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_get(builder, lv, nv, yield_constr);
    eval_ext_circuit_set(builder, lv, nv, yield_constr);

    // Stack constraints.
    // Both operations use memory channel 3. The operations are similar enough that
    // we can constrain both at the same time.
    let filter = lv.op.context_op;
    let channel = lv.mem_channels[3];
    // For get_context, we check if lv.stack_len is 0. For set_context, we check if nv.stack_len is 0.
    // However, for get_context, we can deduce lv.stack_len from nv.stack_len since the operation only pushes.
    let diff = builder.add_const_extension(lv.opcode_bits[0], -F::ONE);
    let stack_len = builder.add_extension(nv.stack_len, diff);
    // Constrain stack_inv_aux. It's 0 if the relevant stack is empty, 1 otherwise.
    {
        let diff = builder.mul_sub_extension(
            stack_len,
            lv.general.stack().stack_inv,
            lv.general.stack().stack_inv_aux,
        );
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    // Enable or disable the channel.
    {
        let diff = builder.sub_extension(lv.general.stack().stack_inv_aux, channel.used);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    let new_filter = builder.mul_extension(filter, lv.general.stack().stack_inv_aux);
    // It's a write for get_context, a read for set_context.
    {
        let diff = builder.sub_extension(channel.is_read, lv.opcode_bits[0]);
        let constr = builder.mul_extension(new_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    // In both cases, next row's context works.
    {
        let diff = builder.sub_extension(channel.addr_context, nv.context);
        let constr = builder.mul_extension(new_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    // Same segment for both.
    {
        let diff = builder.add_const_extension(
            channel.addr_segment,
            -F::from_canonical_u64(Segment::Stack as u64),
        );
        let constr = builder.mul_extension(new_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    // The address is one less than stack_len.
    {
        let addr_virtual = builder.add_const_extension(stack_len, -F::ONE);
        let diff = builder.sub_extension(channel.addr_virtual, addr_virtual);
        let constr = builder.mul_extension(new_filter, diff);
        yield_constr.constraint(builder, constr);
    }
}
