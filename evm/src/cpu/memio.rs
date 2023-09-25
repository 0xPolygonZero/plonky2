use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cpu::stack;

fn get_addr<T: Copy>(lv: &CpuColumnsView<T>) -> (T, T, T) {
    let addr_context = lv.mem_channels[0].value[0];
    let addr_segment = lv.mem_channels[1].value[0];
    let addr_virtual = lv.mem_channels[2].value[0];
    (addr_context, addr_segment, addr_virtual)
}

fn eval_packed_load<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
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

    // Stack constraints
    stack::eval_packed_one(
        lv,
        nv,
        filter,
        stack::MLOAD_GENERAL_OP.unwrap(),
        yield_constr,
    );

    // Check the stack for MLOAD_32BYTES.
    // The second bit (in little-endian) of MLOAD_32BYTES is 0.
    let filter = lv.op.memop_32bytes * (P::ONES - lv.opcode_bits[1]);

    stack::eval_packed_one(
        lv,
        nv,
        filter,
        stack::MLOAD_32BYTES_OP.unwrap(),
        yield_constr,
    );
}

fn eval_ext_circuit_load<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
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

    // Stack constraints
    stack::eval_ext_circuit_one(
        builder,
        lv,
        nv,
        filter,
        stack::MLOAD_GENERAL_OP.unwrap(),
        yield_constr,
    );

    // Check the stack for MLOAD_32BYTES.
    let one = builder.one_extension();
    let mut filter = builder.sub_extension(one, lv.opcode_bits[1]);
    filter = builder.mul_extension(lv.op.memop_32bytes, filter);

    stack::eval_ext_circuit_one(
        builder,
        lv,
        nv,
        filter,
        stack::MLOAD_32BYTES_OP.unwrap(),
        yield_constr,
    );
}

fn eval_packed_store<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
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

    // Stack constraints
    stack::eval_packed_one(
        lv,
        nv,
        filter,
        stack::MSTORE_GENERAL_OP.unwrap(),
        yield_constr,
    );

    // Check the stack for MSTORE_32BYTES.
    // The second bit (in little-endian) of MSTORE_32BYTES is 1.
    let filter = lv.op.memop_32bytes * lv.opcode_bits[1];

    stack::eval_packed_one(
        lv,
        nv,
        filter,
        stack::MSTORE_32BYTES_OP.unwrap(),
        yield_constr,
    );
}

fn eval_ext_circuit_store<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
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

    // Stack constraints
    stack::eval_ext_circuit_one(
        builder,
        lv,
        nv,
        filter,
        stack::MSTORE_GENERAL_OP.unwrap(),
        yield_constr,
    );

    // Check the stack for MSTORE_32BYTES.
    let filter = builder.mul_extension(lv.op.memop_32bytes, lv.opcode_bits[1]);

    stack::eval_ext_circuit_one(
        builder,
        lv,
        nv,
        filter,
        stack::MSTORE_32BYTES_OP.unwrap(),
        yield_constr,
    );
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_load(lv, nv, yield_constr);
    eval_packed_store(lv, nv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_load(builder, lv, nv, yield_constr);
    eval_ext_circuit_store(builder, lv, nv, yield_constr);
}
