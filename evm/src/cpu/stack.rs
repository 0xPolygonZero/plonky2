use std::cmp::max;

use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::ops::OpsColumnsView;
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

pub(crate) const MAX_USER_STACK_SIZE: usize = 1024;

// We check for stack overflows here. An overflow occurs when the stack length is 1025 in user mode,
// which can happen after a non-kernel-only, non-popping, pushing instruction/syscall.
// The check uses `stack_len_bounds_aux`, which is either 0 if next row's `stack_len` is 1025 or
// next row is in kernel mode, or the inverse of `nv.stack_len - 1025` otherwise.
pub(crate) const MIGHT_OVERFLOW: OpsColumnsView<bool> = OpsColumnsView {
    binary_op: false,
    ternary_op: false,
    fp254_op: false,
    eq_iszero: false,
    logic_op: false,
    not_pop: false,
    shift: false,
    jumpdest_keccak_general: false,
    push_prover_input: true, // PROVER_INPUT doesn't require the check, but PUSH does.
    jumps: false,
    pc_push0: true,
    dup_swap: true,
    context_op: false,
    m_op_32bytes: false,
    exit_kernel: true, // Doesn't directly push, but the syscall it's returning from might.
    m_op_general: false,
    syscall: false,
    exception: false,
};

/// Structure to represent opcodes stack behaviours:
/// - number of pops
/// - whether the opcode(s) push
/// - whether unused channels should be disabled.
#[derive(Clone, Copy)]
pub(crate) struct StackBehavior {
    pub(crate) num_pops: usize,
    pub(crate) pushes: bool,
    disable_other_channels: bool,
}

/// `StackBehavior` for unary operations.
pub(crate) const BASIC_UNARY_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 1,
    pushes: true,
    disable_other_channels: true,
});
/// `StackBehavior` for binary operations.
const BASIC_BINARY_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 2,
    pushes: true,
    disable_other_channels: true,
});
/// `StackBehavior` for ternary operations.
const BASIC_TERNARY_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 3,
    pushes: true,
    disable_other_channels: true,
});
/// `StackBehavior` for JUMP.
pub(crate) const JUMP_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 1,
    pushes: false,
    disable_other_channels: false,
});
/// `StackBehavior` for JUMPI.
pub(crate) const JUMPI_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 2,
    pushes: false,
    disable_other_channels: false,
});
/// `StackBehavior` for MLOAD_GENERAL.
pub(crate) const MLOAD_GENERAL_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 1,
    pushes: true,
    disable_other_channels: false,
});

pub(crate) const KECCAK_GENERAL_OP: StackBehavior = StackBehavior {
    num_pops: 2,
    pushes: true,
    disable_other_channels: true,
};

pub(crate) const JUMPDEST_OP: StackBehavior = StackBehavior {
    num_pops: 0,
    pushes: false,
    disable_other_channels: true,
};

// AUDITORS: If the value below is `None`, then the operation must be manually checked to ensure
// that every general-purpose memory channel is either disabled or has its read flag and address
// properly constrained. The same applies  when `disable_other_channels` is set to `false`,
// except the first `num_pops` and the last `pushes as usize` channels have their read flag and
// address constrained automatically in this file.
pub(crate) const STACK_BEHAVIORS: OpsColumnsView<Option<StackBehavior>> = OpsColumnsView {
    binary_op: BASIC_BINARY_OP,
    ternary_op: BASIC_TERNARY_OP,
    fp254_op: BASIC_BINARY_OP,
    eq_iszero: None, // EQ is binary, IS_ZERO is unary.
    logic_op: BASIC_BINARY_OP,
    not_pop: None,
    shift: Some(StackBehavior {
        num_pops: 2,
        pushes: true,
        disable_other_channels: false,
    }),
    jumpdest_keccak_general: None,
    push_prover_input: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: true,
    }),
    jumps: None, // Depends on whether it's a JUMP or a JUMPI.
    pc_push0: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: true,
    }),
    dup_swap: None,
    context_op: None,
    m_op_32bytes: Some(StackBehavior {
        num_pops: 2,
        pushes: true,
        disable_other_channels: false,
    }),
    exit_kernel: Some(StackBehavior {
        num_pops: 1,
        pushes: false,
        disable_other_channels: true,
    }),
    m_op_general: None,
    syscall: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: false,
    }),
    exception: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: false,
    }),
};

/// Stack behavior for EQ.
pub(crate) const EQ_STACK_BEHAVIOR: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 2,
    pushes: true,
    disable_other_channels: true,
});
/// Stack behavior for ISZERO.
pub(crate) const IS_ZERO_STACK_BEHAVIOR: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 1,
    pushes: true,
    disable_other_channels: true,
});

/// Evaluates constraints for one `StackBehavior`.
pub(crate) fn eval_packed_one<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    filter: P,
    stack_behavior: StackBehavior,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // If you have pops.
    if stack_behavior.num_pops > 0 {
        for i in 1..stack_behavior.num_pops {
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

        // You can't have a write of the top of the stack, so you disable the corresponding flag.
        yield_constr.constraint(filter * lv.partial_channel.used);

        // If you also push, you don't need to read the new top of the stack.
        // If you don't:
        // - if the stack isn't empty after the pops, you read the new top from an extra pop.
        // - if not, the extra read is disabled.
        // These are transition constraints: they don't apply to the last row.
        if !stack_behavior.pushes {
            // If stack_len != N...
            let len_diff = lv.stack_len - P::Scalar::from_canonical_usize(stack_behavior.num_pops);
            let new_filter = len_diff * filter;
            // Read an extra element.
            let channel = nv.mem_channels[0];
            yield_constr.constraint_transition(new_filter * (channel.used - P::ONES));
            yield_constr.constraint_transition(new_filter * (channel.is_read - P::ONES));
            yield_constr.constraint_transition(new_filter * (channel.addr_context - nv.context));
            yield_constr.constraint_transition(
                new_filter
                    * (channel.addr_segment
                        - P::Scalar::from_canonical_usize(Segment::Stack.unscale())),
            );
            let addr_virtual = nv.stack_len - P::ONES;
            yield_constr.constraint_transition(new_filter * (channel.addr_virtual - addr_virtual));
            // Constrain `stack_inv_aux`.
            yield_constr.constraint(
                filter
                    * (len_diff * lv.general.stack().stack_inv - lv.general.stack().stack_inv_aux),
            );
            // Disable channel if stack_len == N.
            let empty_stack_filter = filter * (lv.general.stack().stack_inv_aux - P::ONES);
            yield_constr.constraint_transition(empty_stack_filter * channel.used);
        }
    }
    // If the op only pushes, you only need to constrain the top of the stack if the stack isn't empty.
    else if stack_behavior.pushes {
        // If len > 0...
        let new_filter = lv.stack_len * filter;
        // You write the previous top of the stack in memory, in the partial channel.
        // The value will be checked with the CTL.
        let channel = lv.partial_channel;
        yield_constr.constraint(new_filter * (channel.used - P::ONES));
        yield_constr.constraint(new_filter * channel.is_read);
        yield_constr.constraint(new_filter * (channel.addr_context - lv.context));
        yield_constr.constraint(
            new_filter
                * (channel.addr_segment
                    - P::Scalar::from_canonical_usize(Segment::Stack.unscale())),
        );
        let addr_virtual = lv.stack_len - P::ONES;
        yield_constr.constraint(new_filter * (channel.addr_virtual - addr_virtual));
        // Else you disable the channel.
        yield_constr.constraint(
            filter
                * (lv.stack_len * lv.general.stack().stack_inv - lv.general.stack().stack_inv_aux),
        );
        let empty_stack_filter = filter * (lv.general.stack().stack_inv_aux - P::ONES);
        yield_constr.constraint(empty_stack_filter * channel.used);
    }
    // If the op doesn't pop nor push, the top of the stack must not change.
    else {
        yield_constr.constraint(filter * nv.mem_channels[0].used);
        for (limb_old, limb_new) in lv.mem_channels[0]
            .value
            .iter()
            .zip(nv.mem_channels[0].value.iter())
        {
            yield_constr.constraint(filter * (*limb_old - *limb_new));
        }

        // You can't have a write of the top of the stack, so you disable the corresponding flag.
        yield_constr.constraint(filter * lv.partial_channel.used);
    }

    // Unused channels
    if stack_behavior.disable_other_channels {
        // The first channel contains (or not) the top of the stack and is constrained elsewhere.
        for i in max(1, stack_behavior.num_pops)..NUM_GP_CHANNELS - (stack_behavior.pushes as usize)
        {
            let channel = lv.mem_channels[i];
            yield_constr.constraint(filter * channel.used);
        }
    }

    // Constrain new stack length.
    let num_pops = P::Scalar::from_canonical_usize(stack_behavior.num_pops);
    let push = P::Scalar::from_canonical_usize(stack_behavior.pushes as usize);
    yield_constr.constraint_transition(filter * (nv.stack_len - (lv.stack_len - num_pops + push)));
}

/// Evaluates constraints for all opcodes' `StackBehavior`s.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    for (op, stack_behavior, might_overflow) in izip!(
        lv.op.into_iter(),
        STACK_BEHAVIORS.into_iter(),
        MIGHT_OVERFLOW.into_iter()
    ) {
        if let Some(stack_behavior) = stack_behavior {
            eval_packed_one(lv, nv, op, stack_behavior, yield_constr);
        }

        if might_overflow {
            // Check for stack overflow in the next row.
            let diff = nv.stack_len - P::Scalar::from_canonical_usize(MAX_USER_STACK_SIZE + 1);
            let lhs = diff * lv.general.stack().stack_len_bounds_aux;
            let rhs = P::ONES - nv.is_kernel_mode;
            yield_constr.constraint_transition(op * (lhs - rhs));
        }
    }

    // Constrain stack for JUMPDEST.
    let jumpdest_filter = lv.op.jumpdest_keccak_general * lv.opcode_bits[1];
    eval_packed_one(lv, nv, jumpdest_filter, JUMPDEST_OP, yield_constr);

    // Constrain stack for KECCAK_GENERAL.
    let keccak_general_filter = lv.op.jumpdest_keccak_general * (P::ONES - lv.opcode_bits[1]);
    eval_packed_one(
        lv,
        nv,
        keccak_general_filter,
        KECCAK_GENERAL_OP,
        yield_constr,
    );

    // Stack constraints for POP.
    // The only constraints POP has are stack constraints.
    // Since POP and NOT are combined into one flag and they have
    // different stack behaviors, POP needs special stack constraints.
    // Constrain `stack_inv_aux`.
    let len_diff = lv.stack_len - P::Scalar::ONES;
    yield_constr.constraint(
        lv.op.not_pop
            * (len_diff * lv.general.stack().stack_inv - lv.general.stack().stack_inv_aux),
    );

    // If stack_len != 1 and POP, read new top of the stack in nv.mem_channels[0].
    let top_read_channel = nv.mem_channels[0];
    let is_top_read = lv.general.stack().stack_inv_aux * (P::ONES - lv.opcode_bits[0]);

    // Constrain `stack_inv_aux_2`. It contains `stack_inv_aux * (1 - opcode_bits[0])`.
    yield_constr.constraint(lv.op.not_pop * (lv.general.stack().stack_inv_aux_2 - is_top_read));
    let new_filter = lv.op.not_pop * lv.general.stack().stack_inv_aux_2;
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
    // If stack_len == 1 or NOT, disable the channel.
    // If NOT or (len==1 and POP), then `stack_inv_aux_2` = 0.
    yield_constr.constraint(
        lv.op.not_pop * (lv.general.stack().stack_inv_aux_2 - P::ONES) * top_read_channel.used,
    );

    // Disable remaining memory channels.
    for &channel in &lv.mem_channels[1..] {
        yield_constr.constraint(lv.op.not_pop * (lv.opcode_bits[0] - P::ONES) * channel.used);
    }
    yield_constr
        .constraint(lv.op.not_pop * (lv.opcode_bits[0] - P::ONES) * lv.partial_channel.used);

    // Constrain the new stack length for POP.
    yield_constr.constraint_transition(
        lv.op.not_pop * (lv.opcode_bits[0] - P::ONES) * (nv.stack_len - lv.stack_len + P::ONES),
    );
}

/// Circuit version of `eval_packed_one`.
/// Evaluates constraints for one `StackBehavior`.
pub(crate) fn eval_ext_circuit_one<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    filter: ExtensionTarget<D>,
    stack_behavior: StackBehavior,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // If you have pops.
    if stack_behavior.num_pops > 0 {
        for i in 1..stack_behavior.num_pops {
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
                    -F::from_canonical_usize(Segment::Stack.unscale()),
                    filter,
                    channel.addr_segment,
                    filter,
                );
                yield_constr.constraint(builder, constr);
            }
            // Remember that the first read (`i == 1`) is for the second stack element at `stack[stack_len - 1]`.
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

        // You can't have a write of the top of the stack, so you disable the corresponding flag.
        {
            let constr = builder.mul_extension(filter, lv.partial_channel.used);
            yield_constr.constraint(builder, constr);
        }

        // If you also push, you don't need to read the new top of the stack.
        // If you don't:
        // - if the stack isn't empty after the pops, you read the new top from an extra pop.
        // - if not, the extra read is disabled.
        // These are transition constraints: they don't apply to the last row.
        if !stack_behavior.pushes {
            // If stack_len != N...
            let target_num_pops =
                builder.constant_extension(F::from_canonical_usize(stack_behavior.num_pops).into());
            let len_diff = builder.sub_extension(lv.stack_len, target_num_pops);
            let new_filter = builder.mul_extension(filter, len_diff);
            // Read an extra element.
            let channel = nv.mem_channels[0];

            {
                let constr = builder.mul_sub_extension(new_filter, channel.used, new_filter);
                yield_constr.constraint_transition(builder, constr);
            }
            {
                let constr = builder.mul_sub_extension(new_filter, channel.is_read, new_filter);
                yield_constr.constraint_transition(builder, constr);
            }
            {
                let diff = builder.sub_extension(channel.addr_context, nv.context);
                let constr = builder.mul_extension(new_filter, diff);
                yield_constr.constraint_transition(builder, constr);
            }
            {
                let constr = builder.arithmetic_extension(
                    F::ONE,
                    -F::from_canonical_usize(Segment::Stack.unscale()),
                    new_filter,
                    channel.addr_segment,
                    new_filter,
                );
                yield_constr.constraint_transition(builder, constr);
            }
            {
                let diff = builder.sub_extension(channel.addr_virtual, nv.stack_len);
                let constr =
                    builder.arithmetic_extension(F::ONE, F::ONE, new_filter, diff, new_filter);
                yield_constr.constraint_transition(builder, constr);
            }
            // Constrain `stack_inv_aux`.
            {
                let prod = builder.mul_extension(len_diff, lv.general.stack().stack_inv);
                let diff = builder.sub_extension(prod, lv.general.stack().stack_inv_aux);
                let constr = builder.mul_extension(filter, diff);
                yield_constr.constraint(builder, constr);
            }
            // Disable channel if stack_len == N.
            {
                let empty_stack_filter =
                    builder.mul_sub_extension(filter, lv.general.stack().stack_inv_aux, filter);
                let constr = builder.mul_extension(empty_stack_filter, channel.used);
                yield_constr.constraint_transition(builder, constr);
            }
        }
    }
    // If the op only pushes, you only need to constrain the top of the stack if the stack isn't empty.
    else if stack_behavior.pushes {
        // If len > 0...
        let new_filter = builder.mul_extension(lv.stack_len, filter);
        // You write the previous top of the stack in memory, in the last channel.
        // The value will be checked with the CTL
        let channel = lv.partial_channel;
        {
            let constr = builder.mul_sub_extension(new_filter, channel.used, new_filter);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.mul_extension(new_filter, channel.is_read);
            yield_constr.constraint(builder, constr);
        }

        {
            let diff = builder.sub_extension(channel.addr_context, lv.context);
            let constr = builder.mul_extension(new_filter, diff);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.arithmetic_extension(
                F::ONE,
                -F::from_canonical_usize(Segment::Stack.unscale()),
                new_filter,
                channel.addr_segment,
                new_filter,
            );
            yield_constr.constraint(builder, constr);
        }
        {
            let diff = builder.sub_extension(channel.addr_virtual, lv.stack_len);
            let constr = builder.arithmetic_extension(F::ONE, F::ONE, new_filter, diff, new_filter);
            yield_constr.constraint(builder, constr);
        }
        // Else you disable the channel.
        {
            let diff = builder.mul_extension(lv.stack_len, lv.general.stack().stack_inv);
            let diff = builder.sub_extension(diff, lv.general.stack().stack_inv_aux);
            let constr = builder.mul_extension(filter, diff);
            yield_constr.constraint(builder, constr);
        }
        {
            let empty_stack_filter =
                builder.mul_sub_extension(filter, lv.general.stack().stack_inv_aux, filter);
            let constr = builder.mul_extension(empty_stack_filter, channel.used);
            yield_constr.constraint(builder, constr);
        }
    }
    // If the op doesn't pop nor push, the top of the stack must not change.
    else {
        {
            let constr = builder.mul_extension(filter, nv.mem_channels[0].used);
            yield_constr.constraint(builder, constr);
        }
        {
            for (limb_old, limb_new) in lv.mem_channels[0]
                .value
                .iter()
                .zip(nv.mem_channels[0].value.iter())
            {
                let diff = builder.sub_extension(*limb_old, *limb_new);
                let constr = builder.mul_extension(filter, diff);
                yield_constr.constraint(builder, constr);
            }
        }

        // You can't have a write of the top of the stack, so you disable the corresponding flag.
        {
            let constr = builder.mul_extension(filter, lv.partial_channel.used);
            yield_constr.constraint(builder, constr);
        }
    }

    // Unused channels
    if stack_behavior.disable_other_channels {
        // The first channel contains (or not) the top of the stack and is constrained elsewhere.
        for i in max(1, stack_behavior.num_pops)..NUM_GP_CHANNELS - (stack_behavior.pushes as usize)
        {
            let channel = lv.mem_channels[i];
            let constr = builder.mul_extension(filter, channel.used);
            yield_constr.constraint(builder, constr);
        }
    }

    // Constrain new stack length.
    let diff = builder.constant_extension(
        F::Extension::from_canonical_usize(stack_behavior.num_pops)
            - F::Extension::from_canonical_usize(stack_behavior.pushes as usize),
    );
    let diff = builder.sub_extension(lv.stack_len, diff);
    let diff = builder.sub_extension(nv.stack_len, diff);
    let constr = builder.mul_extension(filter, diff);
    yield_constr.constraint_transition(builder, constr);
}

/// Circuit version of `eval_packed`.
/// Evaluates constraints for all opcodes' `StackBehavior`s.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    for (op, stack_behavior, might_overflow) in izip!(
        lv.op.into_iter(),
        STACK_BEHAVIORS.into_iter(),
        MIGHT_OVERFLOW.into_iter()
    ) {
        if let Some(stack_behavior) = stack_behavior {
            eval_ext_circuit_one(builder, lv, nv, op, stack_behavior, yield_constr);
        }

        if might_overflow {
            // Check for stack overflow in the next row.
            let diff = builder.add_const_extension(
                nv.stack_len,
                -F::from_canonical_usize(MAX_USER_STACK_SIZE + 1),
            );
            let prod = builder.mul_add_extension(
                diff,
                lv.general.stack().stack_len_bounds_aux,
                nv.is_kernel_mode,
            );
            let rhs = builder.add_const_extension(prod, -F::ONE);
            let constr = builder.mul_extension(op, rhs);
            yield_constr.constraint_transition(builder, constr);
        }
    }

    // Constrain stack for JUMPDEST.
    let jumpdest_filter = builder.mul_extension(lv.op.jumpdest_keccak_general, lv.opcode_bits[1]);
    eval_ext_circuit_one(builder, lv, nv, jumpdest_filter, JUMPDEST_OP, yield_constr);

    // Constrain stack for KECCAK_GENERAL.
    let one = builder.one_extension();
    let mut keccak_general_filter = builder.sub_extension(one, lv.opcode_bits[1]);
    keccak_general_filter =
        builder.mul_extension(lv.op.jumpdest_keccak_general, keccak_general_filter);
    eval_ext_circuit_one(
        builder,
        lv,
        nv,
        keccak_general_filter,
        KECCAK_GENERAL_OP,
        yield_constr,
    );

    // Stack constraints for POP.
    // The only constraints POP has are stack constraints.
    // Since POP and NOT are combined into one flag and they have
    // different stack behaviors, POP needs special stack constraints.
    // Constrain `stack_inv_aux`.
    {
        let len_diff = builder.add_const_extension(lv.stack_len, F::NEG_ONE);
        let diff = builder.mul_sub_extension(
            len_diff,
            lv.general.stack().stack_inv,
            lv.general.stack().stack_inv_aux,
        );
        let constr = builder.mul_extension(lv.op.not_pop, diff);
        yield_constr.constraint(builder, constr);
    }
    // If stack_len != 4 and MSTORE, read new top of the stack in nv.mem_channels[0].
    let top_read_channel = nv.mem_channels[0];
    let is_top_read = builder.mul_extension(lv.general.stack().stack_inv_aux, lv.opcode_bits[0]);
    let is_top_read = builder.sub_extension(lv.general.stack().stack_inv_aux, is_top_read);
    // Constrain `stack_inv_aux_2`. It contains `stack_inv_aux * opcode_bits[0]`.
    {
        let diff = builder.sub_extension(lv.general.stack().stack_inv_aux_2, is_top_read);
        let constr = builder.mul_extension(lv.op.not_pop, diff);
        yield_constr.constraint(builder, constr);
    }
    let new_filter = builder.mul_extension(lv.op.not_pop, lv.general.stack().stack_inv_aux_2);
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
    // If stack_len == 1 or NOT, disable the channel.
    {
        let diff = builder.mul_sub_extension(
            lv.op.not_pop,
            lv.general.stack().stack_inv_aux_2,
            lv.op.not_pop,
        );
        let constr = builder.mul_extension(diff, top_read_channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Disable remaining memory channels.
    let filter = builder.mul_sub_extension(lv.op.not_pop, lv.opcode_bits[0], lv.op.not_pop);
    for &channel in &lv.mem_channels[1..] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_extension(filter, lv.partial_channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Constrain the new stack length for POP.
    let diff = builder.sub_extension(nv.stack_len, lv.stack_len);
    let mut constr = builder.add_const_extension(diff, F::ONES);
    constr = builder.mul_extension(filter, constr);
    yield_constr.constraint_transition(builder, constr);
}
