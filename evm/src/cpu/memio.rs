use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

fn get_addr<T: Copy>(lv: &CpuColumnsView<T>) -> (T, T, T) {
    let addr_context = lv.mem_channels[0].value[0];
    let addr_segment = lv.mem_channels[1].value[0];
    let addr_virtual = lv.mem_channels[2].value[0];
    (addr_context, addr_segment, addr_virtual)
}

fn eval_packed_load<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // The opcode for MLOAD_GENERAL is 0xfb. If the operation is MLOAD_GENERAL, lv.opcode_bits[0] = 1
    let filter = lv.op.m_op_general * lv.opcode_bits[0];

    let (addr_context, addr_segment, addr_virtual) = get_addr(lv);

    let load_channel = lv.mem_channels[3];
    let push_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];
    yield_constr.constraint(filter * (load_channel.used - P::ONES));
    yield_constr.constraint(filter * (load_channel.is_read - P::ONES));
    yield_constr.constraint(filter * (load_channel.addr_context - addr_context));
    yield_constr.constraint(filter * (load_channel.addr_segment - addr_segment));
    yield_constr.constraint(filter * (load_channel.addr_virtual - addr_virtual));
    for (load_limb, push_limb) in izip!(load_channel.value, push_channel.value) {
        yield_constr.constraint(filter * (load_limb - push_limb));
    }

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[4..NUM_GP_CHANNELS - 1] {
        yield_constr.constraint(filter * channel.used);
    }

    // Stack behavior constraints
    let num_pops = 3;
    let num_operands = num_pops + 1;
    assert!(num_operands <= NUM_GP_CHANNELS);

    // Pops
    for i in 0..num_pops {
        let channel = lv.mem_channels[i];

        yield_constr.constraint(filter * (channel.used - P::ONES));
        yield_constr.constraint(filter * (channel.is_read - P::ONES));

        yield_constr.constraint(filter * (channel.addr_context - lv.context));
        yield_constr.constraint(
            filter * (channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
        );
        // E.g. if `stack_len == 1` and `i == 0`, we want `add_virtual == 0`.
        let addr_virtual = lv.stack_len - P::Scalar::from_canonical_usize(i + 1);
        yield_constr.constraint(filter * (channel.addr_virtual - addr_virtual));
    }

    // Pushes
    yield_constr.constraint(filter * (push_channel.used - P::ONES));
    yield_constr.constraint(filter * push_channel.is_read);

    yield_constr.constraint(filter * (push_channel.addr_context - lv.context));
    yield_constr.constraint(
        filter * (push_channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
    );
    let addr_virtual = lv.stack_len - P::Scalar::from_canonical_usize(num_pops);
    yield_constr.constraint(filter * (push_channel.addr_virtual - addr_virtual));
}

fn eval_ext_circuit_load<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let mut filter = lv.op.m_op_general;
    filter = builder.mul_extension(filter, lv.opcode_bits[0]);

    let (addr_context, addr_segment, addr_virtual) = get_addr(lv);

    let load_channel = lv.mem_channels[3];
    let push_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];
    {
        let constr = builder.mul_sub_extension(filter, load_channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, load_channel.is_read, filter);
        yield_constr.constraint(builder, constr);
    }
    for (channel_field, target) in izip!(
        [
            load_channel.addr_context,
            load_channel.addr_segment,
            load_channel.addr_virtual,
        ],
        [addr_context, addr_segment, addr_virtual]
    ) {
        let diff = builder.sub_extension(channel_field, target);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for (load_limb, push_limb) in izip!(load_channel.value, push_channel.value) {
        let diff = builder.sub_extension(load_limb, push_limb);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[4..NUM_GP_CHANNELS - 1] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Stack behavior constraints
    let num_pops = 3;
    let num_operands = num_pops + 1;
    assert!(num_operands <= NUM_GP_CHANNELS);

    // Pops
    for i in 0..num_pops {
        let channel = lv.mem_channels[i];

        {
            let constr = builder.mul_sub_extension(filter, channel.used, filter);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.mul_sub_extension(filter, channel.is_read, filter);
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
            let constr = builder.arithmetic_extension(
                F::ONE,
                F::from_canonical_usize(i + 1),
                filter,
                diff,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }
    }

    // Pushes
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
        let constr = builder.arithmetic_extension(
            F::ONE,
            F::from_canonical_usize(num_pops),
            filter,
            diff,
            filter,
        );
        yield_constr.constraint(builder, constr);
    }
}

fn eval_packed_store<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.m_op_general * (P::ONES - lv.opcode_bits[0]);

    let (addr_context, addr_segment, addr_virtual) = get_addr(lv);

    let value_channel = lv.mem_channels[3];
    let store_channel = lv.mem_channels[4];
    yield_constr.constraint(filter * (store_channel.used - P::ONES));
    yield_constr.constraint(filter * store_channel.is_read);
    yield_constr.constraint(filter * (store_channel.addr_context - addr_context));
    yield_constr.constraint(filter * (store_channel.addr_segment - addr_segment));
    yield_constr.constraint(filter * (store_channel.addr_virtual - addr_virtual));
    for (value_limb, store_limb) in izip!(value_channel.value, store_channel.value) {
        yield_constr.constraint(filter * (value_limb - store_limb));
    }

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[5..] {
        yield_constr.constraint(filter * channel.used);
    }

    // Stack behavior constraints
    let num_pops = 4;
    assert!(num_pops <= NUM_GP_CHANNELS);

    // Pops
    for i in 0..num_pops {
        let channel = lv.mem_channels[i];

        yield_constr.constraint(filter * (channel.used - P::ONES));
        yield_constr.constraint(filter * (channel.is_read - P::ONES));

        yield_constr.constraint(filter * (channel.addr_context - lv.context));
        yield_constr.constraint(
            filter * (channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
        );
        // E.g. if `stack_len == 1` and `i == 0`, we want `add_virtual == 0`.
        let addr_virtual = lv.stack_len - P::Scalar::from_canonical_usize(i + 1);
        yield_constr.constraint(filter * (channel.addr_virtual - addr_virtual));
    }
}

fn eval_ext_circuit_store<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let mut filter = lv.op.m_op_general;
    let one = builder.one_extension();
    let minus = builder.sub_extension(one, lv.opcode_bits[0]);
    filter = builder.mul_extension(filter, minus);

    let (addr_context, addr_segment, addr_virtual) = get_addr(lv);

    let value_channel = lv.mem_channels[3];
    let store_channel = lv.mem_channels[4];
    {
        let constr = builder.mul_sub_extension(filter, store_channel.used, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_extension(filter, store_channel.is_read);
        yield_constr.constraint(builder, constr);
    }
    for (channel_field, target) in izip!(
        [
            store_channel.addr_context,
            store_channel.addr_segment,
            store_channel.addr_virtual,
        ],
        [addr_context, addr_segment, addr_virtual]
    ) {
        let diff = builder.sub_extension(channel_field, target);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for (value_limb, store_limb) in izip!(value_channel.value, store_channel.value) {
        let diff = builder.sub_extension(value_limb, store_limb);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[5..] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Stack behavior constraints
    let num_pops = 4;
    assert!(num_pops <= NUM_GP_CHANNELS);

    // Pops
    for i in 0..num_pops {
        let channel = lv.mem_channels[i];

        {
            let constr = builder.mul_sub_extension(filter, channel.used, filter);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.mul_sub_extension(filter, channel.is_read, filter);
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
            let constr = builder.arithmetic_extension(
                F::ONE,
                F::from_canonical_usize(i + 1),
                filter,
                diff,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_load(lv, yield_constr);
    eval_packed_store(lv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_load(builder, lv, yield_constr);
    eval_ext_circuit_store(builder, lv, yield_constr);
}
