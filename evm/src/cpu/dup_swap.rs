use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, MemoryChannelView};
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

/// Constrain two channels to have equal values.
fn channels_equal_packed<P: PackedField>(
    filter: P,
    ch_a: &MemoryChannelView<P>,
    ch_b: &MemoryChannelView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    for (limb_a, limb_b) in izip!(ch_a.value, ch_b.value) {
        yield_constr.constraint(filter * (limb_a - limb_b));
    }
}

/// Constrain two channels to have equal values.
fn channels_equal_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    filter: ExtensionTarget<D>,
    ch_a: &MemoryChannelView<ExtensionTarget<D>>,
    ch_b: &MemoryChannelView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    for (limb_a, limb_b) in izip!(ch_a.value, ch_b.value) {
        let diff = builder.sub_extension(limb_a, limb_b);
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
    // Top of the stack is at `addr = lv.stack_len - 1`.
    let addr_virtual = lv.stack_len - P::ONES - offset;
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
        let constr = builder.mul_add_extension(filter, constr, filter);
        yield_constr.constraint(builder, constr);
    }
}

fn eval_packed_dup<P: PackedField>(
    n: P,
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.is_cpu_cycle * lv.op.dup;

    let in_channel = &lv.mem_channels[0];
    let out_channel = &lv.mem_channels[NUM_GP_CHANNELS - 1];

    channels_equal_packed(filter, in_channel, out_channel, yield_constr);

    constrain_channel_packed(true, filter, n, in_channel, lv, yield_constr);
    constrain_channel_packed(
        false,
        filter,
        P::Scalar::NEG_ONE.into(),
        out_channel,
        lv,
        yield_constr,
    );
}

fn eval_ext_circuit_dup<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let neg_one = builder.constant_extension(F::NEG_ONE.into());

    let filter = builder.mul_extension(lv.is_cpu_cycle, lv.op.dup);

    let in_channel = &lv.mem_channels[0];
    let out_channel = &lv.mem_channels[NUM_GP_CHANNELS - 1];

    channels_equal_ext_circuit(builder, filter, in_channel, out_channel, yield_constr);

    constrain_channel_ext_circuit(builder, true, filter, n, in_channel, lv, yield_constr);
    constrain_channel_ext_circuit(
        builder,
        false,
        filter,
        neg_one,
        out_channel,
        lv,
        yield_constr,
    );
}

fn eval_packed_swap<P: PackedField>(
    n: P,
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let n_plus_one = n + P::ONES;

    let filter = lv.is_cpu_cycle * lv.op.swap;

    let in1_channel = &lv.mem_channels[0];
    let in2_channel = &lv.mem_channels[1];
    let out1_channel = &lv.mem_channels[NUM_GP_CHANNELS - 2];
    let out2_channel = &lv.mem_channels[NUM_GP_CHANNELS - 1];

    channels_equal_packed(filter, in1_channel, out1_channel, yield_constr);
    channels_equal_packed(filter, in2_channel, out2_channel, yield_constr);

    constrain_channel_packed(true, filter, P::ZEROS, in1_channel, lv, yield_constr);
    constrain_channel_packed(true, filter, n_plus_one, in2_channel, lv, yield_constr);
    constrain_channel_packed(false, filter, n_plus_one, out1_channel, lv, yield_constr);
    constrain_channel_packed(false, filter, P::ZEROS, out2_channel, lv, yield_constr);
}

fn eval_ext_circuit_swap<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let zero = builder.zero_extension();
    let one = builder.one_extension();
    let n_plus_one = builder.add_extension(n, one);

    let filter = builder.mul_extension(lv.is_cpu_cycle, lv.op.swap);

    let in1_channel = &lv.mem_channels[0];
    let in2_channel = &lv.mem_channels[1];
    let out1_channel = &lv.mem_channels[NUM_GP_CHANNELS - 2];
    let out2_channel = &lv.mem_channels[NUM_GP_CHANNELS - 1];

    channels_equal_ext_circuit(builder, filter, in1_channel, out1_channel, yield_constr);
    channels_equal_ext_circuit(builder, filter, in2_channel, out2_channel, yield_constr);

    constrain_channel_ext_circuit(builder, true, filter, zero, in1_channel, lv, yield_constr);
    constrain_channel_ext_circuit(
        builder,
        true,
        filter,
        n_plus_one,
        in2_channel,
        lv,
        yield_constr,
    );
    constrain_channel_ext_circuit(
        builder,
        false,
        filter,
        n_plus_one,
        out1_channel,
        lv,
        yield_constr,
    );
    constrain_channel_ext_circuit(builder, false, filter, zero, out2_channel, lv, yield_constr);
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let n = lv.opcode_bits[0]
        + lv.opcode_bits[1] * P::Scalar::from_canonical_u64(2)
        + lv.opcode_bits[2] * P::Scalar::from_canonical_u64(4)
        + lv.opcode_bits[3] * P::Scalar::from_canonical_u64(8);

    eval_packed_dup(n, lv, yield_constr);
    eval_packed_swap(n, lv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let n = lv.opcode_bits[..4].iter().enumerate().fold(
        builder.zero_extension(),
        |cumul, (i, &bit)| {
            builder.mul_const_add_extension(F::from_canonical_u64(1 << i), bit, cumul)
        },
    );

    eval_ext_circuit_dup(builder, n, lv, yield_constr);
    eval_ext_circuit_swap(builder, n, lv, yield_constr);
}
