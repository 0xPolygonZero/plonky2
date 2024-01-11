use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use super::cpu_stark::get_addr;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::stack;
use crate::memory::segments::Segment;

const fn get_addr_load<T: Copy>(lv: &CpuColumnsView<T>) -> (T, T, T) {
    get_addr(lv, 0)
}
const fn get_addr_store<T: Copy>(lv: &CpuColumnsView<T>) -> (T, T, T) {
    get_addr(lv, 1)
}

/// Evaluates constraints for MLOAD_GENERAL.
fn eval_packed_load<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // The opcode for MLOAD_GENERAL is 0xfb. If the operation is MLOAD_GENERAL, lv.opcode_bits[0] = 1.
    let filter = lv.op.m_op_general * lv.opcode_bits[0];

    let (addr_context, addr_segment, addr_virtual) = get_addr_load(lv);

    // Check that we are loading the correct value from the correct address.
    let load_channel = lv.mem_channels[1];
    yield_constr.constraint(filter * (load_channel.used - P::ONES));
    yield_constr.constraint(filter * (load_channel.is_read - P::ONES));
    yield_constr.constraint(filter * (load_channel.addr_context - addr_context));
    yield_constr.constraint(filter * (load_channel.addr_segment - addr_segment));
    yield_constr.constraint(filter * (load_channel.addr_virtual - addr_virtual));

    // Constrain the new top of the stack.
    for (&limb_loaded, &limb_new_top) in load_channel
        .value
        .iter()
        .zip(nv.mem_channels[0].value.iter())
    {
        yield_constr.constraint(filter * (limb_loaded - limb_new_top));
    }

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[2..] {
        yield_constr.constraint(filter * channel.used);
    }
    yield_constr.constraint(filter * lv.partial_channel.used);

    // Stack constraints
    stack::eval_packed_one(
        lv,
        nv,
        filter,
        stack::MLOAD_GENERAL_OP.unwrap(),
        yield_constr,
    );
}

/// Circuit version for `eval_packed_load`.
/// Evaluates constraints for MLOAD_GENERAL.
fn eval_ext_circuit_load<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // The opcode for MLOAD_GENERAL is 0xfb. If the operation is MLOAD_GENERAL, lv.opcode_bits[0] = 1.
    let mut filter = lv.op.m_op_general;
    filter = builder.mul_extension(filter, lv.opcode_bits[0]);

    let (addr_context, addr_segment, addr_virtual) = get_addr_load(lv);

    // Check that we are loading the correct value from the correct channel.
    let load_channel = lv.mem_channels[1];
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

    // Constrain the new top of the stack.
    for (&limb_loaded, &limb_new_top) in load_channel
        .value
        .iter()
        .zip(nv.mem_channels[0].value.iter())
    {
        let diff = builder.sub_extension(limb_loaded, limb_new_top);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[2..] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_extension(filter, lv.partial_channel.used);
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
}

/// Evaluates constraints for MSTORE_GENERAL.
fn eval_packed_store<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.m_op_general * (lv.opcode_bits[0] - P::ONES);

    let (addr_context, addr_segment, addr_virtual) = get_addr_store(lv);

    // The value will be checked with the CTL.
    let store_channel = lv.partial_channel;

    yield_constr.constraint(filter * (store_channel.used - P::ONES));
    yield_constr.constraint(filter * store_channel.is_read);
    yield_constr.constraint(filter * (store_channel.addr_context - addr_context));
    yield_constr.constraint(filter * (store_channel.addr_segment - addr_segment));
    yield_constr.constraint(filter * (store_channel.addr_virtual - addr_virtual));

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[2..] {
        yield_constr.constraint(filter * channel.used);
    }

    // Stack constraints.
    // Pops.
    for i in 1..2 {
        let channel = lv.mem_channels[i];

        yield_constr.constraint(filter * (channel.used - P::ONES));
        yield_constr.constraint(filter * (channel.is_read - P::ONES));

        yield_constr.constraint(filter * (channel.addr_context - lv.context));
        yield_constr.constraint(
            filter
                * (channel.addr_segment
                    - P::Scalar::from_canonical_usize(Segment::Stack.unscale())),
        );
        // Remember that the first read (`i == 1`) is for the second stack element at `stack[stack_len - 1]`.
        let addr_virtual = lv.stack_len - P::Scalar::from_canonical_usize(i + 1);
        yield_constr.constraint(filter * (channel.addr_virtual - addr_virtual));
    }
    // Constrain `stack_inv_aux`.
    let len_diff = lv.stack_len - P::Scalar::from_canonical_usize(2);
    yield_constr.constraint(
        lv.op.m_op_general
            * (len_diff * lv.general.stack().stack_inv - lv.general.stack().stack_inv_aux),
    );
    // If stack_len != 2 and MSTORE, read new top of the stack in nv.mem_channels[0].
    let top_read_channel = nv.mem_channels[0];
    let is_top_read = lv.general.stack().stack_inv_aux * (P::ONES - lv.opcode_bits[0]);
    // Constrain `stack_inv_aux_2`. It contains `stack_inv_aux * opcode_bits[0]`.
    yield_constr
        .constraint(lv.op.m_op_general * (lv.general.stack().stack_inv_aux_2 - is_top_read));
    let new_filter = lv.op.m_op_general * lv.general.stack().stack_inv_aux_2;
    yield_constr.constraint_transition(new_filter * (top_read_channel.used - P::ONES));
    yield_constr.constraint_transition(new_filter * (top_read_channel.is_read - P::ONES));
    yield_constr.constraint_transition(new_filter * (top_read_channel.addr_context - nv.context));
    yield_constr.constraint_transition(
        new_filter
            * (top_read_channel.addr_segment
                - P::Scalar::from_canonical_usize(Segment::Stack.unscale())),
    );
    let addr_virtual = nv.stack_len - P::ONES;
    yield_constr.constraint_transition(new_filter * (top_read_channel.addr_virtual - addr_virtual));
    // If stack_len == 2 or MLOAD, disable the channel.
    yield_constr.constraint(
        lv.op.m_op_general * (lv.general.stack().stack_inv_aux - P::ONES) * top_read_channel.used,
    );
    yield_constr.constraint(lv.op.m_op_general * lv.opcode_bits[0] * top_read_channel.used);
}

/// Circuit version of `eval_packed_store`.
/// Evaluates constraints for MSTORE_GENERAL.
fn eval_ext_circuit_store<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter =
        builder.mul_sub_extension(lv.op.m_op_general, lv.opcode_bits[0], lv.op.m_op_general);

    let (addr_context, addr_segment, addr_virtual) = get_addr_store(lv);

    // The value will be checked with the CTL.
    let store_channel = lv.partial_channel;
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

    // Disable remaining memory channels, if any.
    for &channel in &lv.mem_channels[2..] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Stack constraints
    // Pops.
    for i in 1..2 {
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
            let diff = builder.add_const_extension(
                channel.addr_segment,
                -F::from_canonical_usize(Segment::Stack.unscale()),
            );
            let constr = builder.mul_extension(filter, diff);
            yield_constr.constraint(builder, constr);
        }
        // Remember that the first read (`i == 1`) is for the second stack element at `stack[stack_len - 1]`.
        let addr_virtual =
            builder.add_const_extension(lv.stack_len, -F::from_canonical_usize(i + 1));
        let diff = builder.sub_extension(channel.addr_virtual, addr_virtual);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    // Constrain `stack_inv_aux`.
    {
        let len_diff = builder.add_const_extension(lv.stack_len, -F::from_canonical_usize(2));
        let diff = builder.mul_sub_extension(
            len_diff,
            lv.general.stack().stack_inv,
            lv.general.stack().stack_inv_aux,
        );
        let constr = builder.mul_extension(lv.op.m_op_general, diff);
        yield_constr.constraint(builder, constr);
    }
    // If stack_len != 2 and MSTORE, read new top of the stack in nv.mem_channels[0].
    let top_read_channel = nv.mem_channels[0];
    let is_top_read = builder.mul_extension(lv.general.stack().stack_inv_aux, lv.opcode_bits[0]);
    let is_top_read = builder.sub_extension(lv.general.stack().stack_inv_aux, is_top_read);
    // Constrain `stack_inv_aux_2`. It contains `stack_inv_aux * (1 - opcode_bits[0])`.
    {
        let diff = builder.sub_extension(lv.general.stack().stack_inv_aux_2, is_top_read);
        let constr = builder.mul_extension(lv.op.m_op_general, diff);
        yield_constr.constraint(builder, constr);
    }
    let new_filter = builder.mul_extension(lv.op.m_op_general, lv.general.stack().stack_inv_aux_2);
    {
        let constr = builder.mul_sub_extension(new_filter, top_read_channel.used, new_filter);
        yield_constr.constraint_transition(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(new_filter, top_read_channel.is_read, new_filter);
        yield_constr.constraint_transition(builder, constr);
    }
    {
        let diff = builder.sub_extension(top_read_channel.addr_context, nv.context);
        let constr = builder.mul_extension(new_filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }
    {
        let diff = builder.add_const_extension(
            top_read_channel.addr_segment,
            -F::from_canonical_usize(Segment::Stack.unscale()),
        );
        let constr = builder.mul_extension(new_filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }
    {
        let addr_virtual = builder.add_const_extension(nv.stack_len, -F::ONE);
        let diff = builder.sub_extension(top_read_channel.addr_virtual, addr_virtual);
        let constr = builder.mul_extension(new_filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }
    // If stack_len == 2 or MLOAD, disable the channel.
    {
        let diff = builder.mul_sub_extension(
            lv.op.m_op_general,
            lv.general.stack().stack_inv_aux,
            lv.op.m_op_general,
        );
        let constr = builder.mul_extension(diff, top_read_channel.used);
        yield_constr.constraint(builder, constr);
    }
    {
        let mul = builder.mul_extension(lv.op.m_op_general, lv.opcode_bits[0]);
        let constr = builder.mul_extension(mul, top_read_channel.used);
        yield_constr.constraint(builder, constr);
    }
}

/// Evaluates constraints for MLOAD_GENERAL and MSTORE_GENERAL.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_load(lv, nv, yield_constr);
    eval_packed_store(lv, nv, yield_constr);
}

/// Circuit version of `eval_packed`.
/// Evaluates constraints for MLOAD_GENERAL and MSTORE_GENERAL.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_load(builder, lv, nv, yield_constr);
    eval_ext_circuit_store(builder, lv, nv, yield_constr);
}
