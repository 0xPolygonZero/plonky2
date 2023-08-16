use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

fn eval_packed_get<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.get_context;
    let new_stack_top = nv.stack_top;
    yield_constr.constraint(filter * (new_stack_top[0] - lv.context));
    for &limb in &new_stack_top[1..] {
        yield_constr.constraint(filter * limb);
    }
}

fn eval_ext_circuit_get<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = lv.op.get_context;
    let new_stack_top = nv.stack_top;
    {
        let diff = builder.sub_extension(new_stack_top[0], lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &new_stack_top[1..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}

fn eval_packed_set<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.set_context;
    let stack_top = lv.stack_top;
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

    let read_old_stack_top_channel = lv.mem_channels[0];
    let write_old_stack_top_channel = lv.mem_channels[3];
    let read_new_stack_top_channel = lv.mem_channels[4];
    let stack_segment = P::Scalar::from_canonical_u64(Segment::Stack as u64);
    let stack_top_field = P::Scalar::from_canonical_u64(ContextMetadata::StackTop as u64);

    let old_top_to_save_filter = filter * local_sp_dec;
    // The stack top to save is loaded from memory only if old SP is grater than one.
    yield_constr.constraint(old_top_to_save_filter * (read_old_stack_top_channel.used - P::ONES));
    yield_constr
        .constraint(old_top_to_save_filter * (read_old_stack_top_channel.is_read - P::ONES));
    yield_constr.constraint(
        old_top_to_save_filter * (read_old_stack_top_channel.addr_context - lv.context),
    );
    yield_constr.constraint(
        old_top_to_save_filter * (read_old_stack_top_channel.addr_segment - stack_segment),
    );
    yield_constr.constraint(
        old_top_to_save_filter
            * (read_old_stack_top_channel.addr_virtual - (local_sp_dec - P::ONES)),
    );

    // The stack top to save is written to memory only if old SP is greater than one.
    for (limb_write, limb_read) in write_old_stack_top_channel
        .value
        .iter()
        .zip(read_old_stack_top_channel.value.iter())
    {
        yield_constr.constraint(old_top_to_save_filter * (*limb_write - *limb_read));
    }
    yield_constr.constraint(old_top_to_save_filter * (write_old_stack_top_channel.used - P::ONES));
    yield_constr.constraint(old_top_to_save_filter * write_old_stack_top_channel.is_read);
    yield_constr.constraint(
        old_top_to_save_filter * (write_old_stack_top_channel.addr_context - lv.context),
    );
    yield_constr.constraint(
        old_top_to_save_filter * (write_old_stack_top_channel.addr_segment - ctx_metadata_segment),
    );
    yield_constr.constraint(
        old_top_to_save_filter * (write_old_stack_top_channel.addr_virtual - stack_top_field),
    );

    // The next row's stack top is loaded from memory.
    for (limb_channel, limb_top) in read_new_stack_top_channel.value.iter().zip(nv.stack_top) {
        yield_constr.constraint(filter * (*limb_channel - limb_top));
    }
    yield_constr.constraint(filter * (read_new_stack_top_channel.used - P::ONES));
    yield_constr.constraint(filter * (read_new_stack_top_channel.is_read - P::ONES));
    yield_constr.constraint(filter * (read_new_stack_top_channel.addr_context - nv.context));
    yield_constr
        .constraint(filter * (read_new_stack_top_channel.addr_segment - ctx_metadata_segment));
    yield_constr.constraint(filter * (read_new_stack_top_channel.addr_virtual - stack_top_field));

    // TODO: Can you disable channel 0 if old SP = 1 without an extra inv column?
}

fn eval_ext_circuit_set<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = lv.op.set_context;
    let stack_top = lv.stack_top;
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

    let read_old_stack_top_channel = lv.mem_channels[0];
    let write_old_stack_top_channel = lv.mem_channels[3];
    let read_new_stack_top_channel = lv.mem_channels[4];
    let stack_segment =
        builder.constant_extension(F::Extension::from_canonical_u32(Segment::Stack as u32));
    let stack_top_field = builder.constant_extension(F::Extension::from_canonical_u32(
        ContextMetadata::StackTop as u32,
    ));

    let old_top_to_save_filter = builder.mul_extension(filter, local_sp_dec);
    // The stack top to save is loaded from memory only if old SP is grater than one.
    {
        let constr = builder.mul_sub_extension(
            old_top_to_save_filter,
            read_old_stack_top_channel.used,
            old_top_to_save_filter,
        );
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(
            old_top_to_save_filter,
            read_old_stack_top_channel.is_read,
            old_top_to_save_filter,
        );
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(read_old_stack_top_channel.addr_context, lv.context);
        let constr = builder.mul_extension(old_top_to_save_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(read_old_stack_top_channel.addr_segment, stack_segment);
        let constr = builder.mul_extension(old_top_to_save_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let old_stack_top_virt = builder.sub_extension(local_sp_dec, one);
        let diff =
            builder.sub_extension(read_old_stack_top_channel.addr_virtual, old_stack_top_virt);
        let constr = builder.mul_extension(old_top_to_save_filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // The stack top to save is written to memory only if old SP is greater than one.
    for (limb_write, limb_read) in write_old_stack_top_channel
        .value
        .iter()
        .zip(read_old_stack_top_channel.value.iter())
    {
        let diff = builder.sub_extension(*limb_write, *limb_read);
        let constr = builder.mul_extension(old_top_to_save_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(
            old_top_to_save_filter,
            write_old_stack_top_channel.used,
            old_top_to_save_filter,
        );
        yield_constr.constraint(builder, constr);
    }
    {
        let constr =
            builder.mul_extension(old_top_to_save_filter, write_old_stack_top_channel.is_read);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(write_old_stack_top_channel.addr_context, lv.context);
        let constr = builder.mul_extension(old_top_to_save_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(
            write_old_stack_top_channel.addr_segment,
            ctx_metadata_segment,
        );
        let constr = builder.mul_extension(old_top_to_save_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(write_old_stack_top_channel.addr_virtual, stack_top_field);
        let constr = builder.mul_extension(old_top_to_save_filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // The next row's stack top is loaded from memory.
    for (limb_channel, limb_top) in read_new_stack_top_channel.value.iter().zip(nv.stack_top) {
        let diff = builder.sub_extension(*limb_channel, limb_top);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, read_new_stack_top_channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, read_new_stack_top_channel.is_read, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(read_new_stack_top_channel.addr_context, nv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(
            read_new_stack_top_channel.addr_segment,
            ctx_metadata_segment,
        );
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(read_new_stack_top_channel.addr_virtual, stack_top_field);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_get(lv, nv, yield_constr);
    eval_packed_set(lv, nv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_get(builder, lv, nv, yield_constr);
    eval_ext_circuit_set(builder, lv, nv, yield_constr);
}
