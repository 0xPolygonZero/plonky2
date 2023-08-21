use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::columns::MemValue;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, MemoryChannelView};
use crate::memory::segments::Segment;

/// Constrain a channel to have a certain value.
fn channel_value_equal_packed<P: PackedField>(
    filter: P,
    ch: &MemoryChannelView<P>,
    val: &MemValue<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    for (limb_ch, limb) in izip!(ch.value, val) {
        yield_constr.constraint(filter * (limb_ch - *limb));
    }
}

/// Constrain a channel to have a certain value.
fn channel_value_equal_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    filter: ExtensionTarget<D>,
    ch: &MemoryChannelView<ExtensionTarget<D>>,
    val: &MemValue<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    for (limb_ch, limb) in izip!(ch.value, val) {
        let diff = builder.sub_extension(limb_ch, *limb);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
}

/// Set `used`, `is_read`, and address for channel.
///
/// `offset` is the stack index before this instruction is executed, e.g. `0` for the top of the
/// stack.
fn constrain_channel_packed<P: PackedField>(
    is_read: bool,
    filter: P,
    offset: P,
    channel: &MemoryChannelView<P>,
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    yield_constr.constraint(filter * (channel.used - P::ONES));
    yield_constr.constraint(filter * (channel.is_read - P::Scalar::from_bool(is_read)));
    yield_constr.constraint(filter * (channel.addr_context - lv.context));
    yield_constr.constraint(
        filter * (channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
    );
    // Second element of the stack is at `addr = lv.stack_len - 1`.
    let addr_virtual = lv.stack_len - offset;
    yield_constr.constraint(filter * (channel.addr_virtual - addr_virtual));
}

/// Set `used`, `is_read`, and address for channel.
///
/// `offset` is the stack index before this instruction is executed, e.g. `0` for the top of the
/// stack.
fn constrain_channel_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    is_read: bool,
    filter: ExtensionTarget<D>,
    offset: ExtensionTarget<D>,
    channel: &MemoryChannelView<ExtensionTarget<D>>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    {
        let constr = builder.mul_sub_extension(filter, channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = if is_read {
            builder.mul_sub_extension(filter, channel.is_read, filter)
        } else {
            builder.mul_extension(filter, channel.is_read)
        };
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
        let constr = builder.add_extension(channel.addr_virtual, offset);
        let constr = builder.sub_extension(constr, lv.stack_len);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
}

fn eval_packed_dup<P: PackedField>(
    n: P,
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.dup;

    let in_channel = &lv.mem_channels[0];
    let out_channel = &lv.mem_channels[1];

    channel_value_equal_packed(filter, in_channel, &lv.stack_top, yield_constr);
    constrain_channel_packed(false, filter, P::ONES, in_channel, lv, yield_constr);

    // TODO: Warning: Make it a transition constraint? There's a chance this constraint fails
    // if the last line of the trace is a DUP.
    channel_value_equal_packed(filter, out_channel, &nv.stack_top, yield_constr);
    constrain_channel_packed(true, filter, n + P::ONES, out_channel, lv, yield_constr);

    // TODO: Constrain unused channels?
}

fn eval_ext_circuit_dup<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.constant_extension(F::ONE.into());

    let filter = lv.op.dup;

    let in_channel = &lv.mem_channels[0];
    let out_channel = &lv.mem_channels[1];

    channel_value_equal_ext_circuit(builder, filter, in_channel, &lv.stack_top, yield_constr);
    constrain_channel_ext_circuit(builder, false, filter, one, in_channel, lv, yield_constr);

    channel_value_equal_ext_circuit(builder, filter, out_channel, &nv.stack_top, yield_constr);
    let n_plus_one = builder.add_extension(n, one);
    constrain_channel_ext_circuit(
        builder,
        true,
        filter,
        n_plus_one,
        out_channel,
        lv,
        yield_constr,
    );
}

fn eval_packed_swap<P: PackedField>(
    n: P,
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let n_plus_two = n + P::Scalar::from_canonical_u64(2);

    let filter = lv.op.swap;

    let read_channel = &lv.mem_channels[0];
    let write_channel = &lv.mem_channels[1];

    channel_value_equal_packed(filter, read_channel, &nv.stack_top, yield_constr);
    constrain_channel_packed(true, filter, n_plus_two, read_channel, lv, yield_constr);

    channel_value_equal_packed(filter, write_channel, &lv.stack_top, yield_constr);
    constrain_channel_packed(false, filter, n_plus_two, write_channel, lv, yield_constr);

    // TODO: Constrain unused channels?
}

fn eval_ext_circuit_swap<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let two = builder.two_extension();
    let n_plus_two = builder.add_extension(n, two);

    let filter = lv.op.swap;

    let read_channel = &lv.mem_channels[0];
    let write_channel = &lv.mem_channels[1];

    channel_value_equal_ext_circuit(builder, filter, read_channel, &nv.stack_top, yield_constr);
    constrain_channel_ext_circuit(
        builder,
        true,
        filter,
        n_plus_two,
        read_channel,
        lv,
        yield_constr,
    );

    channel_value_equal_ext_circuit(builder, filter, write_channel, &lv.stack_top, yield_constr);
    constrain_channel_ext_circuit(
        builder,
        false,
        filter,
        n_plus_two,
        write_channel,
        lv,
        yield_constr,
    );
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let n = lv.opcode_bits[0]
        + lv.opcode_bits[1] * P::Scalar::from_canonical_u64(2)
        + lv.opcode_bits[2] * P::Scalar::from_canonical_u64(4)
        + lv.opcode_bits[3] * P::Scalar::from_canonical_u64(8);

    eval_packed_dup(n, lv, nv, yield_constr);
    eval_packed_swap(n, lv, nv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let n = lv.opcode_bits[..4].iter().enumerate().fold(
        builder.zero_extension(),
        |cumul, (i, &bit)| {
            builder.mul_const_add_extension(F::from_canonical_u64(1 << i), bit, cumul)
        },
    );

    eval_ext_circuit_dup(builder, n, lv, nv, yield_constr);
    eval_ext_circuit_swap(builder, n, lv, nv, yield_constr);
}
