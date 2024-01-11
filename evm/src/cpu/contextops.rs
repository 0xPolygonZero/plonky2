use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::columns::ops::OpsColumnsView;
use super::cpu_stark::{disable_unused_channels, disable_unused_channels_circuit};
use super::membus::NUM_GP_CHANNELS;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::memory::segments::Segment;
use crate::memory::VALUE_LIMBS;

// If true, the instruction will keep the current context for the next row.
// If false, next row's context is handled manually.
const KEEPS_CONTEXT: OpsColumnsView<bool> = OpsColumnsView {
    binary_op: true,
    ternary_op: true,
    fp254_op: true,
    eq_iszero: true,
    logic_op: true,
    not_pop: true,
    shift: true,
    jumpdest_keccak_general: true,
    push_prover_input: true,
    jumps: true,
    pc_push0: true,
    dup_swap: true,
    context_op: false,
    m_op_32bytes: true,
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

    // context_op is hybrid; we evaluate it separately.
    let is_get_context = lv.op.context_op * (lv.opcode_bits[0] - P::ONES);
    yield_constr.constraint_transition(is_get_context * (nv.context - lv.context));
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

    // context_op is hybrid; we evaluate it separately.
    let is_get_context =
        builder.mul_sub_extension(lv.op.context_op, lv.opcode_bits[0], lv.op.context_op);
    let diff = builder.sub_extension(nv.context, lv.context);
    let constr = builder.mul_extension(is_get_context, diff);
    yield_constr.constraint_transition(builder, constr);
}

/// Evaluates constraints for GET_CONTEXT.
fn eval_packed_get<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // If the opcode is GET_CONTEXT, then lv.opcode_bits[0] = 0.
    let filter = lv.op.context_op * (P::ONES - lv.opcode_bits[0]);
    let new_stack_top = nv.mem_channels[0].value;
    // Context is scaled by 2^64, hence stored in the 3rd limb.
    yield_constr.constraint(filter * (new_stack_top[2] - lv.context));

    for (i, &limb) in new_stack_top.iter().enumerate().filter(|(i, _)| *i != 2) {
        yield_constr.constraint(filter * limb);
    }

    // Constrain new stack length.
    yield_constr.constraint(filter * (nv.stack_len - (lv.stack_len + P::ONES)));

    // Unused channels.
    disable_unused_channels(lv, filter, vec![1], yield_constr);
    yield_constr.constraint(filter * nv.mem_channels[0].used);
}

/// Circuit version of `eval_packed_get`.
/// Evaluates constraints for GET_CONTEXT.
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
    // Context is scaled by 2^64, hence stored in the 3rd limb.
    {
        let diff = builder.sub_extension(new_stack_top[2], lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    for (i, &limb) in new_stack_top.iter().enumerate().filter(|(i, _)| *i != 2) {
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
    disable_unused_channels_circuit(builder, lv, filter, vec![1], yield_constr);
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

    // The next row's context is read from stack_top.
    yield_constr.constraint(filter * (stack_top[2] - nv.context));
    for (i, &limb) in stack_top.iter().enumerate().filter(|(i, _)| *i != 2) {
        yield_constr.constraint(filter * limb);
    }

    // The old SP is decremented (since the new context was popped) and stored in memory.
    // The new SP is loaded from memory.
    // This is all done with CTLs: nothing is constrained here.

    // Constrain stack_inv_aux_2.
    let new_top_channel = nv.mem_channels[0];
    yield_constr.constraint(
        lv.op.context_op
            * (lv.general.stack().stack_inv_aux * lv.opcode_bits[0]
                - lv.general.stack().stack_inv_aux_2),
    );
    // The new top is loaded in memory channel 2, if the stack isn't empty (see eval_packed).
    for (&limb_new_top, &limb_read_top) in new_top_channel
        .value
        .iter()
        .zip(lv.mem_channels[2].value.iter())
    {
        yield_constr.constraint(
            lv.op.context_op * lv.general.stack().stack_inv_aux_2 * (limb_new_top - limb_read_top),
        );
    }

    // Unused channels.
    disable_unused_channels(lv, filter, vec![1], yield_constr);
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

    // The next row's context is read from stack_top.
    {
        let diff = builder.sub_extension(stack_top[2], nv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for (i, &limb) in stack_top.iter().enumerate().filter(|(i, _)| *i != 2) {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }

    // The old SP is decremented (since the new context was popped) and stored in memory.
    // The new SP is loaded from memory.
    // This is all done with CTLs: nothing is constrained here.

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
    // The new top is loaded in memory channel 2, if the stack isn't empty (see eval_packed).
    for (&limb_new_top, &limb_read_top) in new_top_channel
        .value
        .iter()
        .zip(lv.mem_channels[2].value.iter())
    {
        let diff = builder.sub_extension(limb_new_top, limb_read_top);
        let prod = builder.mul_extension(lv.general.stack().stack_inv_aux_2, diff);
        let constr = builder.mul_extension(lv.op.context_op, prod);
        yield_constr.constraint(builder, constr);
    }

    // Unused channels.
    disable_unused_channels_circuit(builder, lv, filter, vec![1], yield_constr);
    {
        let constr = builder.mul_extension(filter, new_top_channel.used);
        yield_constr.constraint(builder, constr);
    }
}

/// Evaluates the constraints for the GET and SET opcodes.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_keep(lv, nv, yield_constr);
    eval_packed_get(lv, nv, yield_constr);
    eval_packed_set(lv, nv, yield_constr);

    // Stack constraints.
    // Both operations use memory channel 2. The operations are similar enough that
    // we can constrain both at the same time.
    let filter = lv.op.context_op;
    let channel = lv.mem_channels[2];
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
        new_filter
            * (channel.addr_segment - P::Scalar::from_canonical_usize(Segment::Stack.unscale())),
    );
    // The address is one less than stack_len.
    let addr_virtual = stack_len - P::ONES;
    yield_constr.constraint(new_filter * (channel.addr_virtual - addr_virtual));
}

/// Circuit version of èval_packed`.
/// Evaluates the constraints for the GET and SET opcodes.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_keep(builder, lv, nv, yield_constr);
    eval_ext_circuit_get(builder, lv, nv, yield_constr);
    eval_ext_circuit_set(builder, lv, nv, yield_constr);

    // Stack constraints.
    // Both operations use memory channel 2. The operations are similar enough that
    // we can constrain both at the same time.
    let filter = lv.op.context_op;
    let channel = lv.mem_channels[2];
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
            -F::from_canonical_usize(Segment::Stack.unscale()),
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
