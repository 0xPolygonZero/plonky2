use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::membus::NUM_GP_CHANNELS;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, MemoryChannelView};
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
        filter * (channel.addr_segment - P::Scalar::from_canonical_usize(Segment::Stack.unscale())),
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
            -F::from_canonical_usize(Segment::Stack.unscale()),
            filter,
            channel.addr_segment,
            filter,
        );
        yield_constr.constraint(builder, constr);
    }
    // Top of the stack is at `addr = lv.stack_len - 1`.
    {
        let constr = builder.add_extension(channel.addr_virtual, offset);
        let constr = builder.sub_extension(constr, lv.stack_len);
        let constr = builder.mul_add_extension(filter, constr, filter);
        yield_constr.constraint(builder, constr);
    }
}

/// Evaluates constraints for DUP.
fn eval_packed_dup<P: PackedField>(
    n: P,
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // DUP opcodes have 0 at the 5-th position, while SWAP opcodes have 1.
    let filter = lv.op.dup_swap * (P::ONES - lv.opcode_bits[4]);

    let write_channel = &lv.mem_channels[1];
    let read_channel = &lv.mem_channels[2];

    // Constrain the input and top of the stack channels to have the same value.
    channels_equal_packed(filter, write_channel, &lv.mem_channels[0], yield_constr);
    // Constrain the output channel's addresses, `is_read` and `used` fields.
    constrain_channel_packed(false, filter, P::ZEROS, write_channel, lv, yield_constr);

    // Constrain the output and top of the stack channels to have the same value.
    channels_equal_packed(filter, read_channel, &nv.mem_channels[0], yield_constr);
    // Constrain the input channel's addresses, `is_read` and `used` fields.
    constrain_channel_packed(true, filter, n, read_channel, lv, yield_constr);

    // Constrain nv.stack_len.
    yield_constr.constraint_transition(filter * (nv.stack_len - lv.stack_len - P::ONES));

    // Disable next top.
    yield_constr.constraint(filter * nv.mem_channels[0].used);
}

/// Circuit version of `eval_packed_dup`.
/// Evaluates constraints for DUP.
fn eval_ext_circuit_dup<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let zero = builder.zero_extension();
    let one = builder.one_extension();
    // DUP opcodes have 0 at the 5-th position, while SWAP opcodes have 1.
    let mut filter = builder.sub_extension(one, lv.opcode_bits[4]);
    filter = builder.mul_extension(lv.op.dup_swap, filter);

    let write_channel = &lv.mem_channels[1];
    let read_channel = &lv.mem_channels[2];

    // Constrain the input and top of the stack channels to have the same value.
    channels_equal_ext_circuit(
        builder,
        filter,
        write_channel,
        &lv.mem_channels[0],
        yield_constr,
    );
    // Constrain the output channel's addresses, `is_read` and `used` fields.
    constrain_channel_ext_circuit(
        builder,
        false,
        filter,
        zero,
        write_channel,
        lv,
        yield_constr,
    );

    // Constrain the output and top of the stack channels to have the same value.
    channels_equal_ext_circuit(
        builder,
        filter,
        read_channel,
        &nv.mem_channels[0],
        yield_constr,
    );
    // Constrain the input channel's addresses, `is_read` and `used` fields.
    constrain_channel_ext_circuit(builder, true, filter, n, read_channel, lv, yield_constr);

    // Constrain nv.stack_len.
    {
        let diff = builder.sub_extension(nv.stack_len, lv.stack_len);
        let constr = builder.mul_sub_extension(filter, diff, filter);
        yield_constr.constraint_transition(builder, constr);
    }

    // Disable next top.
    {
        let constr = builder.mul_extension(filter, nv.mem_channels[0].used);
        yield_constr.constraint(builder, constr);
    }
}

/// Evaluates constraints for SWAP.
fn eval_packed_swap<P: PackedField>(
    n: P,
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let n_plus_one = n + P::ONES;

    // DUP opcodes have 0 at the 5-th position, while SWAP opcodes have 1.
    let filter = lv.op.dup_swap * lv.opcode_bits[4];

    let in1_channel = &lv.mem_channels[0];
    let in2_channel = &lv.mem_channels[1];
    let out_channel = &lv.mem_channels[2];

    // Constrain the first input channel value to be equal to the output channel value.
    channels_equal_packed(filter, in1_channel, out_channel, yield_constr);
    // We set `is_read`, `used` and the address for the first input. The first input is
    // read from the top of the stack, and is therefore not a memory read.
    constrain_channel_packed(false, filter, n_plus_one, out_channel, lv, yield_constr);

    // Constrain the second input channel value to be equal to the new top of the stack.
    channels_equal_packed(filter, in2_channel, &nv.mem_channels[0], yield_constr);
    // We set `is_read`, `used` and the address for the second input.
    constrain_channel_packed(true, filter, n_plus_one, in2_channel, lv, yield_constr);

    // Constrain nv.stack_len.
    yield_constr.constraint(filter * (nv.stack_len - lv.stack_len));

    // Disable next top.
    yield_constr.constraint(filter * nv.mem_channels[0].used);
}

/// Circuit version of `eval_packed_swap`.
/// Evaluates constraints for SWAP.
fn eval_ext_circuit_swap<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();
    let n_plus_one = builder.add_extension(n, one);

    // DUP opcodes have 0 at the 5-th position, while SWAP opcodes have 1.
    let filter = builder.mul_extension(lv.op.dup_swap, lv.opcode_bits[4]);

    let in1_channel = &lv.mem_channels[0];
    let in2_channel = &lv.mem_channels[1];
    let out_channel = &lv.mem_channels[2];

    // Constrain the first input channel value to be equal to the output channel value.
    channels_equal_ext_circuit(builder, filter, in1_channel, out_channel, yield_constr);
    // We set `is_read`, `used` and the address for the first input. The first input is
    // read from the top of the stack, and is therefore not a memory read.
    constrain_channel_ext_circuit(
        builder,
        false,
        filter,
        n_plus_one,
        out_channel,
        lv,
        yield_constr,
    );

    // Constrain the second input channel value to be equal to the new top of the stack.
    channels_equal_ext_circuit(
        builder,
        filter,
        in2_channel,
        &nv.mem_channels[0],
        yield_constr,
    );
    // We set `is_read`, `used` and the address for the second input.
    constrain_channel_ext_circuit(
        builder,
        true,
        filter,
        n_plus_one,
        in2_channel,
        lv,
        yield_constr,
    );

    // Constrain nv.stack_len.
    let diff = builder.sub_extension(nv.stack_len, lv.stack_len);
    let constr = builder.mul_extension(filter, diff);
    yield_constr.constraint(builder, constr);

    // Disable next top.
    {
        let constr = builder.mul_extension(filter, nv.mem_channels[0].used);
        yield_constr.constraint(builder, constr);
    }
}

/// Evaluates the constraints for the DUP and SWAP opcodes.
pub(crate) fn eval_packed<P: PackedField>(
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

    // For both, disable the partial channel.
    yield_constr.constraint(lv.op.dup_swap * lv.partial_channel.used);
}

/// Circuit version of `eval_packed`.
/// Evaluates the constraints for the DUP and SWAP opcodes.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
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

    // For both, disable the partial channel.
    {
        let constr = builder.mul_extension(lv.op.dup_swap, lv.partial_channel.used);
        yield_constr.constraint(builder, constr);
    }
}
